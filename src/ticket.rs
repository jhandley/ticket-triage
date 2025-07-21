use chrono::{DateTime, Utc};
use language_enum::Language;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ProcessingError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicket {
    pub id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub customer_id: String,
}

impl SupportTicket {
    pub fn new(id: String, content: String, timestamp: DateTime<Utc>, customer_id: String) -> Self {
        SupportTicket {
            id,
            content,
            timestamp,
            customer_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessingResult<T> {
    Processing,
    Success(T),
    Error(ProcessingError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTicket {
    pub ticket: SupportTicket,
    pub language: ProcessingResult<Language>,
    pub sentiment: ProcessingResult<SentimentScore>,
    pub category: ProcessingResult<TicketCategory>,
    pub priority: ProcessingResult<TicketPriority>,
}

impl ProcessedTicket {
    pub fn new(ticket: SupportTicket) -> Self {
        ProcessedTicket {
            ticket,
            language: ProcessingResult::Processing,
            sentiment: ProcessingResult::Processing,
            category: ProcessingResult::Processing,
            priority: ProcessingResult::Processing,
        }
    }
    pub fn with_language(mut self, language: ProcessingResult<Language>) -> Self {
        self.language = language;
        self
    }
    pub fn with_sentiment(mut self, sentiment: ProcessingResult<SentimentScore>) -> Self {
        self.sentiment = sentiment;
        self
    }
    pub fn with_category(mut self, category: ProcessingResult<TicketCategory>) -> Self {
        self.category = category;
        self
    }
    pub fn with_priority(mut self, priority: ProcessingResult<TicketPriority>) -> Self {
        self.priority = priority;
        self
    }

    pub fn merge_from(&mut self, other: Self) {
        match other.language {
            ProcessingResult::Processing => {}
            _ => self.language = other.language,
        }

        match other.sentiment {
            ProcessingResult::Processing => {}
            _ => self.sentiment = other.sentiment,
        }

        match other.category {
            ProcessingResult::Processing => {}
            _ => self.category = other.category,
        }

        match other.priority {
            ProcessingResult::Processing => {}
            _ => self.priority = other.priority,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SentimentScore {
    pub label: SentimentLabel,
    pub confidence: f32, // A value between 0.0 and 1.0 indicating the confidence of the sentiment score
}

impl SentimentScore {
    pub fn new(label: SentimentLabel, confidence: f32) -> Self {
        SentimentScore { label, confidence }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SentimentLabel {
    VeryPositive,
    Positive,
    Neutral,
    Negative,
    VeryNegative,
}

impl From<&str> for SentimentLabel {
    fn from(label: &str) -> Self {
        match label {
            "Very Positive" => SentimentLabel::VeryPositive,
            "Positive" => SentimentLabel::Positive,
            "Neutral" => SentimentLabel::Neutral,
            "Negative" => SentimentLabel::Negative,
            "Very Negative" => SentimentLabel::VeryNegative,
            _ => SentimentLabel::Neutral, // Default to Neutral if unknown
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub enum TicketCategory {
    Billing,
    Account,
    General,
    Technical,
    Sales,
    Feedback,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processed_ticket_merge_from() {
        let ticket = SupportTicket::new(
            "test-1".to_string(),
            "Test ticket content".to_string(),
            Utc::now(),
            "customer1".to_string(),
        );
        let mut base = ProcessedTicket::new(ticket.clone());

        let mut update = ProcessedTicket::new(ticket);
        update.language = ProcessingResult::Success(language_enum::Language::Spanish);
        update.sentiment =
            ProcessingResult::Success(SentimentScore::new(SentimentLabel::Negative, 0.9));

        base.merge_from(update);

        assert_eq!(
            base.language,
            ProcessingResult::Success(language_enum::Language::Spanish)
        );
        assert_eq!(
            base.sentiment,
            ProcessingResult::Success(SentimentScore::new(SentimentLabel::Negative, 0.9))
        );
        assert_eq!(base.category, ProcessingResult::Processing);
        assert_eq!(base.priority, ProcessingResult::Processing);
    }
}
