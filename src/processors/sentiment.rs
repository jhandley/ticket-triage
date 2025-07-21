use std::env;

use crate::{
    error::ProcessingError,
    pipeline::{FieldMask, TicketProcessor},
    ticket::{ProcessedTicket, ProcessingResult, SentimentScore},
};
use async_trait::async_trait;
use log::info;
use serde::Deserialize;
use serde_json::json;

pub struct SentimentProcessor {
    client: reqwest::Client,
    api_token: String,
}

#[async_trait]
impl TicketProcessor for SentimentProcessor {
    async fn process(&self, ticket: ProcessedTicket) -> ProcessedTicket {
        info!(
            "SentimentProcessor received event for ticket: {}",
            ticket.ticket.id
        );

        let ticket_id = ticket.ticket.id.clone();
        let sentiment = match self.analyze_sentiment(&ticket.ticket.content).await {
            Ok(sentiment) => ProcessingResult::Success(sentiment),
            Err(err) => ProcessingResult::Error(err),
        };
        let result = ticket.with_sentiment(sentiment);

        info!(
            "SentimentProcessor finished processing ticket: {}",
            ticket_id
        );
        result
    }

    fn required_fields(&self) -> FieldMask {
        FieldMask::empty()
    }

    fn output_fields(&self) -> FieldMask {
        FieldMask::SENTIMENT
    }
}

impl SentimentProcessor {
    pub fn new() -> Result<Self, ProcessingError> {
        let api_token = env::var("HUGGING_FACE_API_TOKEN").map_err(|_| {
            ProcessingError::SentimentAnalysis("HUGGING_FACE_API_TOKEN not set".to_string())
        })?;

        Ok(SentimentProcessor {
            client: reqwest::Client::new(),
            api_token,
        })
    }

    async fn analyze_sentiment(&self, text: &str) -> Result<SentimentScore, ProcessingError> {
        let url = "https://router.huggingface.co/hf-inference/models/tabularisai/multilingual-sentiment-analysis";
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "inputs": text,
                "parameters": { "top_k": 1 }
            }))
            .send()
            .await
            .map_err(|e| ProcessingError::SentimentAnalysis(e.to_string()))?;

        response
            .error_for_status_ref()
            .map_err(|e| ProcessingError::SentimentAnalysis(format!("HTTP error: {}", e)))?;

        // Parse the response to extract sentiment score. Response will be in the format: [[{"label":"Very Positive","score":0.6382827162742615}]]
        // Label will be one of "Very Positive", "Positive", "Neutral", "Negative", "Very Negative" and score is a float between 0.0 and 1.0.
        let parsed: Vec<Vec<HuggingFaceResponse>> = response
            .json()
            .await
            .map_err(|e| ProcessingError::SentimentAnalysis(e.to_string()))?;

        let hugging_face_sentiment = parsed.first().and_then(|v| v.first()).ok_or_else(|| {
            ProcessingError::SentimentAnalysis("Invalid response format".to_string())
        })?;

        let sentiment = SentimentScore {
            label: hugging_face_sentiment.label.as_str().into(),
            confidence: hugging_face_sentiment.score,
        };
        Ok(sentiment)
    }
}

#[derive(Debug, Deserialize)]
struct HuggingFaceResponse {
    label: String,
    score: f32,
}
