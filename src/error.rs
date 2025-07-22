use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessingError {
    #[error("Failed to process the ticket: {0}")]
    TicketProcessingError(String),

    #[error("Invalid ticket data: {0}")]
    InvalidTicketData(String),

    #[error("Network error occurred: {0}")]
    NetworkError(String),

    #[error("Failed to detect language")]
    LanguageDetectionError(),

    #[error("Sentiment analysis failed: {0}")]
    SentimentAnalysis(String),

    #[error("Classification error: {0}")]
    ClassificationError(String),

    #[error("Priority calculation failed: {0}")]
    PriorityCalculationError(String),

    #[error("Unknown error occurred: {0}")]
    UnknownError(String),
}

impl From<reqwest::Error> for ProcessingError {
    fn from(err: reqwest::Error) -> Self {
        Self::NetworkError(err.to_string())
    }
}
