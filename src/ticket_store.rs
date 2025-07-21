use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::ticket::ProcessedTicket;

#[derive(Debug, Clone)]
pub struct TicketStore {
    tickets: Arc<RwLock<HashMap<String, ProcessedTicket>>>,
}

impl Default for TicketStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TicketStore {
    pub fn new() -> Self {
        TicketStore {
            tickets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_ticket(&self, ticket: ProcessedTicket) {
        self.tickets
            .write()
            .await
            .insert(ticket.ticket.id.clone(), ticket);
    }

    pub async fn get_ticket(&self, id: &str) -> Option<ProcessedTicket> {
        self.tickets.read().await.get(id).cloned()
    }

    pub async fn remove_ticket(&self, id: &str) {
        self.tickets.write().await.remove(id);
    }

    pub async fn update_ticket<F>(&self, id: &str, updater: F) -> Option<ProcessedTicket>
    where
        F: FnOnce(&mut ProcessedTicket),
    {
        let mut tickets = self.tickets.write().await;
        tickets.get_mut(id).map(|ticket| {
            updater(ticket);
            ticket.clone()
        })
    }
}
