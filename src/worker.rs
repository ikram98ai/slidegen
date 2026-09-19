//! worker — SQS-driven job runner for Slidegen
//! (subject TOC analysis, interactive chapter packages, inbound ingest).
//!
//! Same binary on Lambda (SQS event source) and Fargate (long-poll).
//! Per-message failures are reported back so SQS retries only the jobs
//! that failed (requires `ReportBatchItemFailures` on Lambda).

use std::sync::Arc;

use aws_lambda_events::event::sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use slidegen::config::Settings;
use slidegen::db::Database;
use slidegen::services::bg_tasks::{Job, JobQueue};
use slidegen::services::events::EventBus;
use slidegen::services::qdrant::QdrantClient;
use slidegen::services::retrieval::RetrievalService;
use slidegen::services::{AIService, BackgroundTasksService, StorageService};

fn build_service(
    db: Arc<Database>,
    storage: Arc<StorageService>,
    ai: Arc<AIService>,
    queue: Option<JobQueue>,
    events: Option<EventBus>,
    auto_generate: bool,
    retrieval: Option<Arc<RetrievalService>>,
) -> BackgroundTasksService {
    BackgroundTasksService::with_events(db, storage, ai, queue, events, auto_generate, retrieval)
}

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
                tracing::error!(error = %e, message_id = %message_id, body = %body, "Dropping malformed job message");
                continue;
            }
        };

        tracing::info!(?job, message_id = %message_id, "Worker: running job");

        if let Err(e) = bg_tasks.run_job(job).await {
            tracing::error!(error = format!("{e:#}"), message_id = %message_id, "Worker: job failed");
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
    let events = EventBus::from_settings(&settings).await;
    let auto_generate = settings.auto_generate_chapters;
    // The worker must be able to enqueue the next chapter after TOC / a package.
    let queue = match settings.jobs_queue_url.clone() {
        Some(url) => Some(JobQueue::new(&settings, url).await),
        None => None,
    };
    let retrieval = Some(Arc::new(RetrievalService::new(
        db.clone(),
        storage.clone(),
        ai.clone(),
        QdrantClient::from_settings(&settings),
    )));
    let bg_tasks = build_service(db, storage, ai, queue, events, auto_generate, retrieval);

    lambda_runtime::run(service_fn(|event: LambdaEvent<SqsEvent>| {
        handle_event(event, &bg_tasks)
    }))
    .await
}
