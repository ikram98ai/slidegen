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

const DEFAULT_WAIT_SECS: i32 = 20;
const DEFAULT_VISIBILITY_SECS: i32 = 900;
const DEFAULT_MAX_MESSAGES: i32 = 1;

fn running_on_lambda() -> bool {
    std::env::var("AWS_LAMBDA_RUNTIME_API").is_ok()
}

fn env_i32(name: &str, default: i32) -> i32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

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

/// `None` means the body was malformed and should be dropped (not retried).
fn parse_job(body: &str, message_id: &str) -> Option<Job> {
    match serde_json::from_str(body) {
        Ok(job) => Some(job),
        Err(e) => {
            tracing::error!(error = %e, message_id, body, "Dropping malformed job message");
            None
        }
    }
}

async fn run_parsed_job(
    bg_tasks: &BackgroundTasksService,
    job: Job,
    message_id: &str,
) -> Result<(), anyhow::Error> {
    tracing::info!(?job, message_id, "Worker: running job");
    bg_tasks.run_job(job).await
}

async fn handle_event(
    event: LambdaEvent<SqsEvent>,
    bg_tasks: &BackgroundTasksService,
) -> Result<SqsBatchResponse, Error> {
    let mut batch_item_failures = Vec::new();

    for record in event.payload.records {
        let message_id = record.message_id.unwrap_or_default();
        let body = record.body.unwrap_or_default();
        let Some(job) = parse_job(&body, &message_id) else {
            continue;
        };
        if let Err(e) = run_parsed_job(bg_tasks, job, &message_id).await {
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

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}

async fn poll_loop(queue: &JobQueue, bg_tasks: &BackgroundTasksService) -> Result<(), Error> {
    let wait = env_i32("WORKER_WAIT_SECONDS", DEFAULT_WAIT_SECS);
    let visibility = env_i32("WORKER_VISIBILITY_SECONDS", DEFAULT_VISIBILITY_SECS);
    let max = env_i32("WORKER_MAX_MESSAGES", DEFAULT_MAX_MESSAGES).clamp(1, 10);

    tracing::info!(
        wait_seconds = wait,
        visibility_seconds = visibility,
        max_messages = max,
        "Worker: long-polling SQS (Fargate / local)"
    );

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("Worker: shutdown requested; exiting poll loop");
                break;
            }
            received = queue.receive(max, wait, visibility) => {
                let messages = match received {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!(error = format!("{e:#}"), "Worker: receive failed");
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                };
                for msg in messages {
                    let Some(job) = parse_job(&msg.body, &msg.message_id) else {
                        if let Err(e) = queue.delete(&msg.receipt_handle).await {
                            tracing::warn!(error = format!("{e:#}"), "Failed to delete malformed message");
                        }
                        continue;
                    };
                    match run_parsed_job(bg_tasks, job, &msg.message_id).await {
                        Ok(()) => {
                            if let Err(e) = queue.delete(&msg.receipt_handle).await {
                                tracing::error!(error = format!("{e:#}"), "Failed to delete completed message");
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                error = format!("{e:#}"),
                                message_id = %msg.message_id,
                                "Worker: job failed; leaving message for retry"
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
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
    let bg_tasks = build_service(
        db,
        storage,
        ai,
        queue.clone(),
        events,
        auto_generate,
        retrieval,
    );

    if running_on_lambda() {
        tracing::info!("Worker: Lambda SQS event source");
        lambda_runtime::run(service_fn(|event: LambdaEvent<SqsEvent>| {
            handle_event(event, &bg_tasks)
        }))
        .await
    } else {
        let Some(queue) = queue else {
            return Err("JOBS_QUEUE_URL must be set when not running on Lambda".into());
        };
        poll_loop(&queue, &bg_tasks).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_job_drops_malformed() {
        assert!(parse_job("not-json", "m1").is_none());
    }

    #[test]
    fn parse_job_accepts_generate_chapter() {
        let body = r#"{"type":"generate_chapter","user_id":"u","subject_id":"s","chapter_id":"c"}"#;
        let job = parse_job(body, "m1").expect("job");
        assert!(matches!(job, Job::GenerateChapter { .. }));
    }

    #[test]
    fn lambda_detection_reads_runtime_api() {
        // Absence is the local/Fargate path; the env var is set only inside Lambda.
        let was = std::env::var("AWS_LAMBDA_RUNTIME_API").ok();
        unsafe { std::env::remove_var("AWS_LAMBDA_RUNTIME_API") };
        assert!(!running_on_lambda());
        if let Some(v) = was {
            unsafe { std::env::set_var("AWS_LAMBDA_RUNTIME_API", v) };
        }
    }
}
