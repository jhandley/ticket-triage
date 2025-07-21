# Ticket Triage

A Rust-based customer support ticket processing system that automatically analyzes and categorizes support tickets using AI-powered processors. This is a toy project to help me review Rust.

## Overview

Analyzes customer support tickets through multiple processors to extract valuable insights including:

- **Language Detection**: Automatically identifies the language of the ticket content
- **Sentiment Analysis**: Determines the emotional tone and confidence level
- **Category Classification**: Categorizes tickets into predefined types (Billing, Technical, Account, etc.)
- **Priority Scoring**: Assigns a priority score based on sentiment and category

## Installation

### Prerequisites

- Rust 2024 edition or later
- OpenAI API key (for classification processor)
- Hugging Face API token (for language detection)

### Setup

1. Clone the repository:
```bash
git clone <repository-url>
cd ticket-triage
```

2. Create a `.env` file with your OpenAI API key:
```bash
echo "OPENAI_API_KEY=your_api_key_here" > .env
echo "HUGGING_FACE_API_TOKEN=your_hugging_face_token_here" >> .env
```

3. Build the project:
```bash
cargo build
```

## Usage

```bash
cargo run
```

## Configuration

### Environment Variables

- `OPENAI_API_KEY`: Required for the classification processor
- `HUGGING_FACE_API_TOKEN`: Required for the language detection processor

## Development

### Adding Custom Processors

To create a custom processor, implement the `TicketProcessor` trait:

```rust
use async_trait::async_trait;
use crate::{
    pipeline::{FieldMask, TicketProcessor},
    ticket::{ProcessedTicket, ProcessingResult},
};

pub struct CustomProcessor;

#[async_trait]
impl TicketProcessor for CustomProcessor {
    async fn process(&self, ticket: ProcessedTicket) -> ProcessedTicket {
        // Your custom processing logic here
        // Example: Add a custom field to the ticket
        let result = ProcessingResult::Success("custom_value".to_string());
        ticket.with_custom_field(result)
    }

    fn required_fields(&self) -> FieldMask {
        // Specify which fields this processor needs to be available
        // before it can run. Use FieldMask::empty() if no dependencies.
        FieldMask::LANGUAGE | FieldMask::SENTIMENT
    }

    fn output_fields(&self) -> FieldMask {
        // Specify which fields this processor produces/updates
        // This helps the pipeline determine processing order
        FieldMask::CATEGORY
    }
}
```

The `TicketProcessor` trait has three methods:

- `process()`: Contains your processing logic and returns the updated ticket
- `required_fields()`: Specifies dependencies - which fields must be completed before this processor runs
- `output_fields()`: Specifies which fields this processor produces, helping determine execution order

Available `FieldMask` values:
- `FieldMask::LANGUAGE`: Language detection
- `FieldMask::SENTIMENT`: Sentiment analysis  
- `FieldMask::CATEGORY`: Category classification
- `FieldMask::PRIORITY`: Priority scoring

Use `FieldMask::empty()` for no dependencies and combine flags with `|` for multiple fields.

### Testing

Run the test suite:

```bash
cargo test
```

### Building Documentation

Generate and view documentation:

```bash
cargo doc --open
```

## License

This project is licensed under the [MIT License](LICENSE) - see the LICENSE file for details.


