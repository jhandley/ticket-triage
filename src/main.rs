use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use std::io;
use std::sync::Arc;
use ticket_triage::{
    pipeline::TicketPipeline,
    processors::{
        classification::ClassificationProcessor, language::LanguageProcessor,
        priority::PriorityProcessor, sentiment::SentimentProcessor,
    },
    ticket::{ProcessingResult, SupportTicket},
};

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    // Read ticket content from stdin
    println!("Please enter your support ticket content:");
    let mut input = String::new();
    if let Err(e) = io::stdin().read_line(&mut input) {
        eprintln!("Error reading from stdin: {}", e);
        return;
    }

    let ticket_content = input.trim().to_string();
    if ticket_content.is_empty() {
        eprintln!("No ticket content provided!");
        return;
    }

    let timestamp: DateTime<Utc> = Utc::now();
    let customer_id = "customer1".to_string();

    let ticket = SupportTicket::new("t1".to_string(), ticket_content, timestamp, customer_id);

    let pipeline = Arc::new(
        TicketPipeline::default()
            .with_processor(Arc::new(LanguageProcessor))
            .with_processor(Arc::new(SentimentProcessor::new().unwrap()))
            .with_processor(Arc::new(ClassificationProcessor::new().unwrap()))
            .with_processor(Arc::new(PriorityProcessor::new().unwrap())),
    );

    // Start the pipeline processing loop in the background
    let pipeline_clone = Arc::clone(&pipeline);
    tokio::spawn(async move {
        pipeline_clone
            .run()
            .await
            .expect("Failed to start pipeline");
    });

    // Give the pipeline a moment to set up subscribers
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let processed_ticket = pipeline
        .process_ticket(ticket)
        .await
        .expect("Failed to process ticket");

    // Print results in a nice human-readable format
    println!("\n{}", "=".repeat(60));
    println!("🎫 TICKET ANALYSIS RESULTS");
    println!("{}", "=".repeat(60));
    println!("📝 Content: {}", processed_ticket.ticket.content);
    println!(
        "🕒 Timestamp: {}",
        processed_ticket
            .ticket
            .timestamp
            .format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("👤 Customer ID: {}", processed_ticket.ticket.customer_id);

    match &processed_ticket.language {
        ProcessingResult::Success(language) => {
            println!("🌍 Language: {:?}", language);
        }
        ProcessingResult::Processing => {
            println!("🌍 Language: Processing...");
        }
        ProcessingResult::Error(err) => {
            println!("🌍 Language: Error - {:?}", err);
        }
    }

    match &processed_ticket.sentiment {
        ProcessingResult::Success(sentiment) => {
            println!(
                "😊 Sentiment: {:?} (confidence: {:.2})",
                sentiment.label, sentiment.confidence
            );
        }
        ProcessingResult::Processing => {
            println!("😊 Sentiment: Processing...");
        }
        ProcessingResult::Error(err) => {
            println!("😊 Sentiment: Error - {:?}", err);
        }
    }

    match &processed_ticket.category {
        ProcessingResult::Success(category) => {
            println!("📂 Category: {:?}", category);
        }
        ProcessingResult::Processing => {
            println!("📂 Category: Processing...");
        }
        ProcessingResult::Error(err) => {
            println!("📂 Category: Error - {:?}", err);
        }
    }

    match &processed_ticket.priority {
        ProcessingResult::Success(priority) => {
            println!("⚡ Priority: {:?}", priority);
        }
        ProcessingResult::Processing => {
            println!("⚡ Priority: Processing...");
        }
        ProcessingResult::Error(err) => {
            println!("⚡ Priority: Error - {:?}", err);
        }
    }

    println!("{}", "=".repeat(60));
}
