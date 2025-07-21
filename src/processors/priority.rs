use crate::{
    error::ProcessingError,
    pipeline::{FieldMask, TicketProcessor},
    ticket::{
        ProcessedTicket, ProcessingResult, SentimentLabel, SentimentScore, TicketCategory,
        TicketPriority,
    },
};
use async_trait::async_trait;
use log::info;

pub struct PriorityProcessor;

#[async_trait]
impl TicketProcessor for PriorityProcessor {
    async fn process(&self, ticket: ProcessedTicket) -> ProcessedTicket {
        info!(
            "PriorityProcessor received event for ticket: {}",
            ticket.ticket.id
        );

        let ticket_id = ticket.ticket.id.clone();
        // Priority is determined by the sentiment and category
        // A more negative sentiment boosts priority and certain categories also boost priority
        let priority_result = self.calculate_priority(&ticket);
        let result = ticket.with_priority(priority_result);

        info!(
            "PriorityProcessor finished processing ticket: {}",
            ticket_id
        );
        result
    }

    fn required_fields(&self) -> FieldMask {
        FieldMask::SENTIMENT | FieldMask::CATEGORY
    }

    fn output_fields(&self) -> FieldMask {
        FieldMask::PRIORITY
    }
}

impl PriorityProcessor {
    pub fn new() -> Result<Self, ProcessingError> {
        Ok(Self)
    }

    fn calculate_priority(&self, ticket: &ProcessedTicket) -> ProcessingResult<TicketPriority> {
        match (&ticket.sentiment, &ticket.category) {
            (ProcessingResult::Success(sentiment), ProcessingResult::Success(category)) => {
                let priority = calculate_priority_from_sentiment_and_category(sentiment, category);
                ProcessingResult::Success(priority)
            }
            _ => ProcessingResult::Error(ProcessingError::PriorityCalculationError(
                "Insufficient data to calculate priority - both sentiment and category are required".to_string(),
            )),
        }
    }
}

/// Returns the base priority score for a ticket category
/// Higher scores indicate higher priority
pub fn get_category_priority_weight(category: &TicketCategory) -> u8 {
    match category {
        TicketCategory::Billing => 7,   // High - affects customer money
        TicketCategory::Account => 6,   // High - affects customer access
        TicketCategory::Technical => 8, // Very High - system issues
        TicketCategory::Sales => 4,     // Medium - business opportunity
        TicketCategory::Feedback => 2,  // Low - nice to have
        TicketCategory::General => 3,   // Low-Medium - general inquiries
        TicketCategory::Other => 3,     // Low-Medium - unknown issues
    }
}

/// Returns the priority multiplier for a sentiment label
/// More negative sentiment increases priority
pub fn get_sentiment_priority_multiplier(sentiment_label: &SentimentLabel) -> f32 {
    match sentiment_label {
        SentimentLabel::VeryNegative => 1.5,
        SentimentLabel::Negative => 1.3,
        SentimentLabel::Neutral => 1.0,
        SentimentLabel::Positive => 0.8,
        SentimentLabel::VeryPositive => 0.6,
    }
}

/// Calculate priority based on sentiment and category using a heuristic
pub fn calculate_priority_from_sentiment_and_category(
    sentiment: &SentimentScore,
    category: &TicketCategory,
) -> TicketPriority {
    // Base score from category (0-10 scale)
    let category_weight = get_category_priority_weight(category) as f32;

    // Apply sentiment multiplier
    let sentiment_multiplier = get_sentiment_priority_multiplier(&sentiment.label);

    // Apply confidence boost for high-confidence negative sentiments
    let confidence_boost = if matches!(
        sentiment.label,
        SentimentLabel::Negative | SentimentLabel::VeryNegative
    ) && sentiment.confidence > 0.8
    {
        1.2
    } else {
        1.0
    };

    // Calculate final score
    let final_score = category_weight * sentiment_multiplier * confidence_boost;

    // Map score to priority levels
    match final_score {
        s if s >= 10.0 => TicketPriority::Critical, // Very high urgency
        s if s >= 7.0 => TicketPriority::High,      // High urgency
        s if s >= 4.0 => TicketPriority::Medium,    // Medium urgency
        _ => TicketPriority::Low,                   // Low urgency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticket::{SentimentLabel, SentimentScore, TicketCategory};

    #[test]
    fn test_priority_calculation_critical() {
        // Very negative sentiment + technical issue = Critical
        let sentiment = SentimentScore::new(SentimentLabel::VeryNegative, 0.9);
        let category = TicketCategory::Technical;

        let priority = calculate_priority_from_sentiment_and_category(&sentiment, &category);
        assert_eq!(priority, TicketPriority::Critical);
    }

    #[test]
    fn test_priority_calculation_high() {
        // Negative sentiment + billing issue = High
        let sentiment = SentimentScore::new(SentimentLabel::Negative, 0.8);
        let category = TicketCategory::Billing;

        let priority = calculate_priority_from_sentiment_and_category(&sentiment, &category);
        assert_eq!(priority, TicketPriority::High);
    }

    #[test]
    fn test_priority_calculation_medium() {
        // Neutral sentiment + account issue = Medium
        let sentiment = SentimentScore::new(SentimentLabel::Neutral, 0.7);
        let category = TicketCategory::Account;

        let priority = calculate_priority_from_sentiment_and_category(&sentiment, &category);
        assert_eq!(priority, TicketPriority::Medium);
    }

    #[test]
    fn test_priority_calculation_low() {
        // Positive sentiment + feedback = Low
        let sentiment = SentimentScore::new(SentimentLabel::Positive, 0.8);
        let category = TicketCategory::Feedback;

        let priority = calculate_priority_from_sentiment_and_category(&sentiment, &category);
        assert_eq!(priority, TicketPriority::Low);
    }

    #[test]
    fn test_confidence_boost() {
        // High confidence negative sentiment should boost priority
        let high_conf_sentiment = SentimentScore::new(SentimentLabel::Negative, 0.95);
        let low_conf_sentiment = SentimentScore::new(SentimentLabel::Negative, 0.6);
        let category = TicketCategory::General;

        let high_priority =
            calculate_priority_from_sentiment_and_category(&high_conf_sentiment, &category);
        let low_priority =
            calculate_priority_from_sentiment_and_category(&low_conf_sentiment, &category);

        // High confidence should result in higher priority
        assert!(matches!(
            high_priority,
            TicketPriority::Medium | TicketPriority::High
        ));
        assert!(matches!(
            low_priority,
            TicketPriority::Low | TicketPriority::Medium
        ));
    }

    #[test]
    fn test_category_priority_weights() {
        assert_eq!(get_category_priority_weight(&TicketCategory::Technical), 8);
        assert_eq!(get_category_priority_weight(&TicketCategory::Billing), 7);
        assert_eq!(get_category_priority_weight(&TicketCategory::Account), 6);
        assert_eq!(get_category_priority_weight(&TicketCategory::Sales), 4);
        assert_eq!(get_category_priority_weight(&TicketCategory::General), 3);
        assert_eq!(get_category_priority_weight(&TicketCategory::Other), 3);
        assert_eq!(get_category_priority_weight(&TicketCategory::Feedback), 2);
    }

    #[test]
    fn test_sentiment_priority_multipliers() {
        assert_eq!(
            get_sentiment_priority_multiplier(&SentimentLabel::VeryNegative),
            1.5
        );
        assert_eq!(
            get_sentiment_priority_multiplier(&SentimentLabel::Negative),
            1.3
        );
        assert_eq!(
            get_sentiment_priority_multiplier(&SentimentLabel::Neutral),
            1.0
        );
        assert_eq!(
            get_sentiment_priority_multiplier(&SentimentLabel::Positive),
            0.8
        );
        assert_eq!(
            get_sentiment_priority_multiplier(&SentimentLabel::VeryPositive),
            0.6
        );
    }
}
