use async_openai::{Client, config::OpenAIConfig, types::*};
use schemars::{JsonSchema, schema_for};

use crate::{
    error::ProcessingError,
    pipeline::{FieldMask, TicketProcessor},
    ticket::{ProcessedTicket, ProcessingResult, TicketCategory},
};
use async_trait::async_trait;
use log::info;
use serde::{Deserialize, Serialize};

pub struct ClassificationProcessor {
    client: Client<OpenAIConfig>,
}

#[async_trait]
impl TicketProcessor for ClassificationProcessor {
    async fn process(&self, ticket: ProcessedTicket) -> ProcessedTicket {
        info!(
            "ClassificationProcessor received event for ticket: {}",
            ticket.ticket.id
        );

        let ticket_id = ticket.ticket.id.clone();
        let category = match self.classify_ticket(&ticket.ticket.content).await {
            Ok(category) => ProcessingResult::Success(category),
            Err(e) => ProcessingResult::Error(e),
        };
        let result = ticket.with_category(category);

        info!(
            "ClassificationProcessor finished processing ticket: {}",
            ticket_id
        );
        result
    }

    fn required_fields(&self) -> FieldMask {
        FieldMask::empty()
    }

    fn output_fields(&self) -> FieldMask {
        FieldMask::CATEGORY
    }
}

impl ClassificationProcessor {
    pub fn new() -> Result<Self, ProcessingError> {
        Ok(Self {
            client: Client::new(),
        })
    }

    async fn classify_ticket(&self, text: &str) -> Result<TicketCategory, ProcessingError> {
        let prompt = self.build_prompt(text);

        let schema = schema_for!(OpenAIClassificationResponse);
        let mut response_schema = serde_json::to_value(schema).map_err(|e| {
            ProcessingError::ClassificationError(format!("Schema generation error: {}", e))
        })?;

        // Add additionalProperties: false to all object schemas
        Self::add_additional_properties_false(&mut response_schema);

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4.1-nano")
            .messages(vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(prompt),
                    name: None,
                },
            )])
            .max_tokens(50_u32)
            .temperature(0.0)
            .response_format(ResponseFormat::JsonSchema {
                json_schema: ResponseFormatJsonSchema {
                    description: None,
                    name: "classification".to_string(),
                    schema: Some(response_schema),
                    strict: Some(true),
                },
            })
            .build()
            .map_err(|e| ProcessingError::ClassificationError(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| ProcessingError::ClassificationError(e.to_string()))?;

        let response: OpenAIClassificationResponse = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .and_then(|content| serde_json::from_str(content).ok())
            .ok_or_else(|| {
                ProcessingError::ClassificationError(
                    "Failed to parse classification response".to_string(),
                )
            })?;

        Ok(response.category)
    }

    fn build_prompt(&self, ticket_content: &str) -> String {
        format!(
            r#"Read the customer support message below and classify it into one of the specified categories.
Output the category and your confidence in the classification as a number between 0.0 and 1.0. Format the result as JSON following the given schema.
{{"category": "CategoryName", "confidence": 0.95}}

Examples:
- "My payment failed and I can't access my account" -> {{"category": "Billing", "confidence": 0.95}}
- "The app crashes when I try to upload" -> {{"category": "Technical", "confidence": 0.90}}
- "I forgot my password" -> {{"category": "Account", "confidence": 0.85}}
- "Do you have a mobile app?" -> {{"category": "General", "confidence": 0.80}}

Ticket: "{ticket_content}""#,
            ticket_content = ticket_content
        )
    }

    /// Recursively adds `additionalProperties: false` to all object schemas in a JSON schema.
    ///
    /// OpenAI's structured output API requires that all object schemas explicitly set
    /// `additionalProperties: false` for strict mode to work properly. The schemars crate
    /// doesn't include this property by default, so we need to add it manually to ensure
    /// the OpenAI API accepts our schema.
    fn add_additional_properties_false(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(obj) => {
                // If this is an object schema (has "type": "object"), add additionalProperties: false
                if let Some(serde_json::Value::String(type_str)) = obj.get("type") {
                    if type_str == "object" {
                        obj.insert(
                            "additionalProperties".to_string(),
                            serde_json::Value::Bool(false),
                        );
                    }
                }

                // Recursively process all nested values
                for (_, val) in obj.iter_mut() {
                    Self::add_additional_properties_false(val);
                }
            }
            serde_json::Value::Array(arr) => {
                // Process array elements
                for val in arr.iter_mut() {
                    Self::add_additional_properties_false(val);
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct OpenAIClassificationResponse {
    category: TicketCategory,
    confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_add_additional_properties_false() {
        // Test case 1: Simple object schema
        let mut schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            }
        });

        ClassificationProcessor::add_additional_properties_false(&mut schema);

        assert_eq!(schema["additionalProperties"], json!(false));

        // Test case 2: Nested object schema
        let mut nested_schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                },
                "settings": {
                    "type": "object",
                    "properties": {
                        "theme": {"type": "string"}
                    }
                }
            }
        });

        ClassificationProcessor::add_additional_properties_false(&mut nested_schema);

        // Root object should have additionalProperties: false
        assert_eq!(nested_schema["additionalProperties"], json!(false));
        // Nested objects should also have additionalProperties: false
        assert_eq!(
            nested_schema["properties"]["user"]["additionalProperties"],
            json!(false)
        );
        assert_eq!(
            nested_schema["properties"]["settings"]["additionalProperties"],
            json!(false)
        );

        // Test case 3: Array with object items
        let mut array_schema = json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"}
                }
            }
        });

        ClassificationProcessor::add_additional_properties_false(&mut array_schema);

        // Array itself shouldn't have additionalProperties, but the object items should
        assert!(array_schema.get("additionalProperties").is_none());
        assert_eq!(array_schema["items"]["additionalProperties"], json!(false));

        // Test case 4: Non-object types should be unchanged
        let mut string_schema = json!({
            "type": "string"
        });

        ClassificationProcessor::add_additional_properties_false(&mut string_schema);

        assert!(string_schema.get("additionalProperties").is_none());

        // Test case 5: Complex nested structure
        let mut complex_schema = json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "nested": {
                                "type": "object",
                                "properties": {
                                    "value": {"type": "string"}
                                }
                            }
                        }
                    }
                }
            }
        });

        ClassificationProcessor::add_additional_properties_false(&mut complex_schema);

        // Root object
        assert_eq!(complex_schema["additionalProperties"], json!(false));
        // Object in array items
        assert_eq!(
            complex_schema["properties"]["data"]["items"]["additionalProperties"],
            json!(false)
        );
        // Deeply nested object
        assert_eq!(
            complex_schema["properties"]["data"]["items"]["properties"]["nested"]["additionalProperties"],
            json!(false)
        );
    }
}
