use axum::{Json, Router, routing::get};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod db;
mod error;
mod models;
mod services;

use crate::config::Settings;
use crate::db::Database;
use crate::services::{AIService, BackgroundTasksService, StorageService};

use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
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
            )
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
        api::chapters::create_chapter,
        api::chapters::update_chapter,
        api::chapters::delete_chapter,
        api::chapters::generate_slides,
        api::chapters::get_chapter_slides,
        api::chapters::update_slide,
        api::chapters::delete_slide,
    ),
    components(
        schemas(
            crate::models::UserCreate,
            crate::models::UserResponse,
            api::auth::LoginReq,
            api::auth::Token,
            api::auth::RefreshTokenReq,
            crate::models::SubjectResponse,
            crate::models::SubjectType,
            crate::models::SubjectCreate,
            crate::models::SubjectUpdate,
            crate::models::ChapterCreate,
            crate::models::ChapterUpdate,
            crate::models::ChapterResponse,
            crate::models::SlideCreate,
            crate::models::SlideUpdate,
            crate::models::SlideResponse,
            crate::api::chapters::GenerativeResponse,
            crate::models::UserUpdate,
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

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub db: Arc<Database>,
    pub storage: Arc<StorageService>,
    pub ai: Arc<AIService>,
    pub bg_tasks: Arc<BackgroundTasksService>,
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

    let db = Arc::new(Database::new(&settings).await);
    let storage = Arc::new(StorageService::new(&settings).await);
    let ai = Arc::new(AIService::new(&settings));
    let bg_tasks = Arc::new(BackgroundTasksService::new(
        db.clone(),
        storage.clone(),
        ai.clone(),
    ));

    let shared_state = Arc::new(AppState {
        settings,
        db,
        storage,
        ai,
        bg_tasks,
    });

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(root))
        .route("/health", get(health_check))
        .nest("/api/auth", api::auth::router())
        .nest("/api/users", api::users::router())
        .nest("/api/subjects", api::subjects::router())
        .nest("/api/chapters", api::chapters::router())
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    if std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok() {
        tracing::info!("Running on AWS Lambda");
        lambda_http::run(app)
            .await
            .map_err(|e| anyhow::anyhow!("Lambda run error: {}", e))?;
    } else {
        let listener = TcpListener::bind("0.0.0.0:8000").await?;
        tracing::info!("listening on {}", listener.local_addr()?);
        axum::serve(listener, app).await?;
    }

    Ok(())
}
