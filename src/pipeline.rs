use std::sync::Arc;

use async_trait::async_trait;
use log::{info, warn};
use tokio::sync::broadcast;

use crate::{
    error::ProcessingError,
    ticket::{ProcessedTicket, ProcessingResult, SupportTicket},
    ticket_store::TicketStore,
};
use bitflags::bitflags;

#[async_trait]
pub trait TicketProcessor: Sync + Send {
    async fn process(&self, ticket: ProcessedTicket) -> ProcessedTicket;

    fn required_fields(&self) -> FieldMask;

    /// Returns the fields that this processor produces/updates
    fn output_fields(&self) -> FieldMask;
}

pub struct TicketPipeline {
    processors: Vec<Arc<dyn TicketProcessor>>,
    ticket_store: Arc<TicketStore>,
    event_sender: Arc<broadcast::Sender<TicketUpdateEvent>>,
}

impl Default for TicketPipeline {
    fn default() -> Self {
        Self {
            processors: Vec::new(),
            ticket_store: Arc::new(TicketStore::default()),
            event_sender: Arc::new(broadcast::channel::<TicketUpdateEvent>(16).0),
        }
    }
}

impl TicketPipeline {
    pub fn with_processor(mut self, processor: Arc<dyn TicketProcessor>) -> Self {
        self.processors.push(processor);
        self
    }

    pub async fn run(&self) -> Result<(), ProcessingError> {
        if self.processors.is_empty() {
            return Err(ProcessingError::TicketProcessingError(
                "No processors configured".to_string(),
            ));
        }

        for processor in &self.processors {
            let ticket_store_clone = Arc::clone(&self.ticket_store);
            let event_sender_clone = Arc::clone(&self.event_sender);
            let processor_clone = Arc::clone(processor);

            tokio::spawn(async move {
                let mut rx = event_sender_clone.subscribe();
                while let Ok(event) = rx.recv().await {
                    let required_fields = processor_clone.required_fields();

                    if let Some(ticket) = ticket_store_clone.get_ticket(&event.ticket_id).await {
                        let current_fields = FieldMask::from(&ticket);

                        // Only process if:
                        // 1. All required fields are available (dependencies are met)
                        // 2. The field(s) this processor produces are NOT yet set (it hasn't run yet)
                        let dependencies_met = current_fields.contains(required_fields);
                        let processor_output_fields = processor_clone.output_fields();
                        let not_yet_processed = !current_fields.intersects(processor_output_fields);

                        if dependencies_met && not_yet_processed {
                            info!(
                                "Processor starting processing for ticket: {} with completed fields: {:?}, required: {:?}, produces: {:?}",
                                event.ticket_id,
                                current_fields,
                                required_fields,
                                processor_output_fields
                            );

                            let ticket_id = ticket.ticket.id.clone();
                            let updated_ticket = processor_clone.process(ticket).await;

                            let updated_fields = FieldMask::from(&updated_ticket);
                            let updated = ticket_store_clone
                                .update_ticket(&ticket_id, |t| {
                                    t.merge_from(updated_ticket);
                                })
                                .await;

                            if updated.is_some() {
                                info!(
                                    "Processor completed processing for ticket: {} with updated fields: {:?}",
                                    ticket_id, updated_fields
                                );
                                let _ = event_sender_clone.send(TicketUpdateEvent {
                                    ticket_id,
                                    completed_fields: updated_fields,
                                });
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn process_ticket(
        &self,
        ticket: SupportTicket,
    ) -> Result<ProcessedTicket, ProcessingError> {
        info!("Starting to process ticket: {}", ticket.id);
        let processed_ticket = ProcessedTicket::new(ticket);
        self.ticket_store.add_ticket(processed_ticket.clone()).await;
        self.event_sender
            .send(TicketUpdateEvent {
                ticket_id: processed_ticket.ticket.id.clone(),
                completed_fields: FieldMask::empty(),
            })
            .map_err(|_| {
                ProcessingError::TicketProcessingError("Failed to send event".to_string())
            })?;

        let result = self
            .wait_for_processing(processed_ticket.ticket.id.clone())
            .await;

        match &result {
            Ok(_) => info!(
                "Successfully completed processing for ticket: {}",
                processed_ticket.ticket.id
            ),
            Err(e) => warn!(
                "Failed to process ticket {}: {:?}",
                processed_ticket.ticket.id, e
            ),
        }

        result
    }

    async fn wait_for_processing(
        &self,
        ticket_id: String,
    ) -> Result<ProcessedTicket, ProcessingError> {
        let mut rx = self.event_sender.subscribe();

        loop {
            match rx.recv().await {
                Ok(TicketUpdateEvent {
                    ticket_id: id,
                    completed_fields,
                }) if id == ticket_id && completed_fields == FieldMask::all() => {
                    break;
                }
                Ok(_) => continue,
                Err(_) => {
                    return Err(ProcessingError::TicketProcessingError(
                        "Event channel closed".to_string(),
                    ));
                }
            }
        }

        self.ticket_store.get_ticket(&ticket_id).await.ok_or(
            ProcessingError::TicketProcessingError("Ticket not found".to_string()),
        )
    }
}

#[derive(Debug, Clone)]
struct TicketUpdateEvent {
    ticket_id: String,
    completed_fields: FieldMask,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FieldMask: u32 {
        const LANGUAGE = 0b0001;
        const SENTIMENT = 0b0010;
        const CATEGORY = 0b0100;
        const PRIORITY = 0b1000;
    }
}

impl From<&ProcessedTicket> for FieldMask {
    fn from(ticket: &ProcessedTicket) -> Self {
        let mut mask = FieldMask::empty();
        match ticket.language {
            ProcessingResult::Processing => {}
            _ => mask.insert(FieldMask::LANGUAGE),
        }
        match ticket.sentiment {
            ProcessingResult::Processing => {}
            _ => mask.insert(FieldMask::SENTIMENT),
        }
        match ticket.category {
            ProcessingResult::Processing => {}
            _ => mask.insert(FieldMask::CATEGORY),
        }
        match ticket.priority {
            ProcessingResult::Processing => {}
            _ => mask.insert(FieldMask::PRIORITY),
        }
        mask
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono::Utc;

    // Helper function to create a test ticket
    fn create_test_ticket() -> SupportTicket {
        SupportTicket::new(
            "test-1".to_string(),
            "Test ticket content".to_string(),
            Utc::now(),
            "customer1".to_string(),
        )
    }

    #[test]
    fn test_field_mask_from_processed_ticket() {
        let ticket = create_test_ticket();
        let mut processed = ProcessedTicket::new(ticket);

        // Initially all fields are Processing, so mask should be empty
        let mask = FieldMask::from(&processed);
        assert_eq!(mask, FieldMask::empty());

        // Set language to success
        processed.language = ProcessingResult::Success(language_enum::Language::English);
        let mask = FieldMask::from(&processed);
        assert!(mask.contains(FieldMask::LANGUAGE));
        assert!(!mask.contains(FieldMask::SENTIMENT));

        // Set sentiment to error
        processed.sentiment =
            ProcessingResult::Error(ProcessingError::InvalidTicketData("test".to_string()));
        let mask = FieldMask::from(&processed);
        assert!(mask.contains(FieldMask::LANGUAGE));
        assert!(mask.contains(FieldMask::SENTIMENT));
        assert!(!mask.contains(FieldMask::CATEGORY));
    }
}
