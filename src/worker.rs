//! worker — SQS-driven Lambda that executes Slidegen background jobs
//! (subject TOC analysis, slide generation, interactive chapter packages).
//!
//! The API Lambda enqueues `Job` messages (see `services::bg_tasks::Job`);
//! this binary is deployed as a second Lambda function with the SQS queue as
//! its event source. Per-message failures are reported back so SQS retries
//! only the jobs that failed (requires `ReportBatchItemFailures` on the event
//! source mapping).

use std::sync::Arc;

use aws_lambda_events::event::sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use slidegen::config::Settings;
use slidegen::db::Database;
use slidegen::services::bg_tasks::Job;
use slidegen::services::{AIService, BackgroundTasksService, StorageService};

async fn handle_event(
    event: LambdaEvent<SqsEvent>,
    bg_tasks: &BackgroundTasksService,
) -> Result<SqsBatchResponse, Error> {
    let mut batch_item_failures = Vec::new();

    for record in event.payload.records {
        let message_id = record.message_id.unwrap_or_default();
        let body = record.body.unwrap_or_default();

        let job: Job = match serde_json::from_str(&body) {
            Ok(job) => job,
            Err(e) => {
                // Malformed messages are dropped, not retried: they will never
                // parse, and endless redelivery would just block the queue.
                tracing::error!(error = %e, message_id = %message_id, body = %body, "Dropping malformed job message");
                continue;
            }
        };

        tracing::info!(?job, message_id = %message_id, "Worker: running job");

        if let Err(e) = bg_tasks.run_job(job).await {
            tracing::error!(error = format!("{e:#}"), message_id = %message_id, "Worker: job failed");
            // BatchItemFailure is #[non_exhaustive]; construct via Default.
            let mut failure = BatchItemFailure::default();
            failure.item_identifier = message_id;
            batch_item_failures.push(failure);
        }
    }

    let mut response = SqsBatchResponse::default();
    response.batch_item_failures = batch_item_failures;
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "slidegen=info,worker=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .init();

    let settings = Settings::new();
    let db = Arc::new(Database::new(&settings).await);
    let storage = Arc::new(StorageService::new(&settings).await);
    let ai = Arc::new(AIService::new(&settings));

    // The worker consumes jobs; it never enqueues, so no queue is wired in.
    let bg_tasks = BackgroundTasksService::new(db, storage, ai, None);

    lambda_runtime::run(service_fn(|event: LambdaEvent<SqsEvent>| {
        handle_event(event, &bg_tasks)
    }))
    .await
}
