use axum::{Json, Router, routing::get};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use slidegen::AppState;
use slidegen::api;
use slidegen::config::Settings;
use slidegen::db::Database;
use slidegen::services::bg_tasks::JobQueue;
use slidegen::services::events::EventBus;
use slidegen::services::qdrant::QdrantClient;
use slidegen::services::retrieval::RetrievalService;
use slidegen::services::{AIService, BackgroundTasksService, StorageService};

use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
            components.add_security_scheme(
                "apiKeyAuth",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Key"))),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    paths(
        root,
        health_check,
        api::auth::register,
        api::auth::login,
        api::auth::refresh_token,
        api::users::get_myprofile,
        api::users::get_user,
        api::users::get_user_subjects,
        api::users::update_user,
        api::users::delete_user,
        api::subjects::list_subjects,
        api::subjects::upload_subject,
        api::subjects::get_subject,
        api::subjects::get_subject_chapters,
        api::subjects::update_subject,
        api::subjects::delete_subject,
        api::subjects::reprocess_subject,
        api::chapters::create_chapter,
        api::chapters::update_chapter,
        api::chapters::delete_chapter,
        api::chapters::generate_slides,
        api::chapters::get_chapter_embed,
        api::chapters::get_chapter_slides,
        api::chapters::update_slide,
        api::chapters::delete_slide,
        api::jobs::get_job,
        api::v1::upload_book,
        api::v1::ingest_book,
        api::v1::get_job,
        api::v1::get_book,
        api::v1::list_chapters,
        api::v1::get_chapter,
        api::v1::get_chapter_embed,
        api::v1::generate_chapter,
        api::v1::search_book,
        api::v1::ask_chapter,
        api::v1::get_scene,
        api::v1::get_paragraph,
    ),
    components(
        schemas(
            slidegen::models::UserCreate,
            slidegen::models::UserResponse,
            api::auth::LoginReq,
            api::auth::Token,
            api::auth::RefreshTokenReq,
            slidegen::models::SubjectResponse,
            slidegen::models::SubjectDetailResponse,
            slidegen::models::SubjectType,
            slidegen::models::SubjectCreate,
            slidegen::models::SubjectUpdate,
            slidegen::models::ChapterCreate,
            slidegen::models::ChapterUpdate,
            slidegen::models::ChapterResponse,
            slidegen::models::SlideCreate,
            slidegen::models::SlideUpdate,
            slidegen::models::SlideResponse,
            api::chapters::GenerativeResponse,
            api::chapters::ChapterEmbedResponse,
            slidegen::models::UserUpdate,
            slidegen::models::JobResponse,
            slidegen::models::IngestBookRequest,
            slidegen::models::IngestBookResponse,
            api::v1::ServiceChapterResponse,
            slidegen::services::retrieval::SearchRequest,
            slidegen::services::retrieval::SearchResponse,
            slidegen::services::retrieval::SearchHit,
            slidegen::services::retrieval::AskRequest,
            slidegen::services::retrieval::AskResponse,
            slidegen::models::Citation,
            slidegen::models::SceneSpec,
        ),
    ),
    tags(
        (name = "Slidegen", description = "Education Platform APIs")
    )
)]
struct ApiDoc;

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Returns API version and message")
    )
)]
async fn root() -> Json<serde_json::Value> {
    Json(json!({"message": "Slidegen Education Platform API", "version": "1.0.0"}))
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check")
    )
)]
async fn health_check() -> Json<serde_json::Value> {
    Json(json!({"status": "healthy"}))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "slidegen=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::new();
    let on_lambda = std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok();

    let db = Arc::new(Database::new(&settings).await);
    let storage = Arc::new(StorageService::new(&settings).await);
    let ai = Arc::new(AIService::new(&settings));

    // With a queue configured, background jobs are sent to SQS and handled by
    // the worker Lambda. Without one they run in-process via tokio::spawn,
    // which is fine locally but stalls on Lambda once the response is sent.
    let job_queue = match settings.jobs_queue_url.clone() {
        Some(url) => Some(JobQueue::new(&settings, url).await),
        None => {
            if on_lambda {
                tracing::warn!(
                    "JOBS_QUEUE_URL is not set: background jobs will run in-process \
                     and may stall when the Lambda environment freezes"
                );
            }
            None
        }
    };

    let events = EventBus::from_settings(&settings).await;
    let auto_generate = settings.auto_generate_chapters;
    let retrieval = Arc::new(RetrievalService::new(
        db.clone(),
        storage.clone(),
        ai.clone(),
        QdrantClient::from_settings(&settings),
    ));
    let bg_tasks = Arc::new(BackgroundTasksService::with_events(
        db.clone(),
        storage.clone(),
        ai.clone(),
        job_queue,
        events,
        auto_generate,
        Some(retrieval.clone()),
    ));

    let shared_state = Arc::new(AppState {
        settings,
        db,
        storage,
        ai,
        bg_tasks,
        retrieval,
    });

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(root))
        .route("/health", get(health_check))
        .nest("/api/auth", api::auth::router())
        .nest("/api/users", api::users::router())
        .nest("/api/subjects", api::subjects::router())
        .nest("/api/chapters", api::chapters::router())
        .nest("/api/jobs", api::jobs::router())
        .nest("/api/v1", api::v1::router())
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    if on_lambda {
        tracing::info!("Running on AWS Lambda");
        lambda_http::run(app)
            .await
            .map_err(|e| anyhow::anyhow!("Lambda run error: {}", e))?;
    } else {
        let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
        let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
        tracing::info!("listening on {}", listener.local_addr()?);
        axum::serve(listener, app).await?;
    }

    Ok(())
}
