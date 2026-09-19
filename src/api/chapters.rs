use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::AppState;
use crate::api::extractors::AuthUser;
use crate::error::AppError;
use crate::models::{
    Chapter, ChapterCreate, ChapterResponse, ChapterUpdate, SlideResponse, SlideUpdate,
};

pub fn router() -> Router<Arc<AppState>> {
    use axum::routing::delete;
    Router::new()
        .route("/", post(create_chapter))
        .route("/{subject_id}/{chapter_id}", patch(update_chapter))
        .route("/{subject_id}/{chapter_id}", delete(delete_chapter))
        .route(
            "/{subject_id}/{chapter_id}/slides/generate",
            post(generate_slides),
        )
        .route("/{subject_id}/{chapter_id}/generate", post(generate_chapter))
        .route("/{subject_id}/{chapter_id}/embed", get(get_chapter_embed))
        .route("/{chapter_id}/slides", get(get_chapter_slides))
        .route("/{chapter_id}/slides/{slide_id}", patch(update_slide))
        .route("/{chapter_id}/slides/{slide_id}", delete(delete_slide))
}

#[utoipa::path(
    post,
    path = "/api/chapters",
    request_body = ChapterCreate,
    responses(
        (status = 201, description = "Successfully created chapter", body = ChapterResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Subject not found")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn create_chapter(
    State(state): State<Arc<AppState>>,
    AuthUser(current_user): AuthUser,
    Json(payload): Json<ChapterCreate>,
) -> Result<Json<ChapterResponse>, AppError> {
    // Get subject
    let subject = match state.db.get_subject(&payload.subject_id).await {
        Ok(Some(s)) => s,
        _ => return Err(AppError::NotFound("Subject not found".to_string())),
    };

    if current_user.id != subject.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to add chapters to this subject".to_string(),
        ));
    }

    let chapter = Chapter {
        id: Uuid::new_v4().to_string(),
        subject_id: subject.id,
        user_id: current_user.id,
        title: payload.title,
        page_start: payload.page_start,
        page_end: payload.page_end,
        order_index: payload.order_index,
        processing_status: Some("completed".to_string()),
        processed_slides: None,
        total_slides: None,
        job_id: None,
        package_key: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    state
        .db
        .save_chapter(&chapter)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(Json(ChapterResponse::from(chapter)))
}

#[utoipa::path(
    patch,
    path = "/api/chapters/{subject_id}/{chapter_id}",
    params(
        ("subject_id" = String, Path, description = "Subject ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    request_body = ChapterUpdate,
    responses(
        (status = 200, description = "Successfully updated chapter", body = ChapterResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn update_chapter(
    State(state): State<Arc<AppState>>,
    Path((subject_id, chapter_id)): Path<(String, String)>,
    AuthUser(current_user): AuthUser,
    Json(payload): Json<ChapterUpdate>,
) -> Result<Json<ChapterResponse>, AppError> {
    let subject = match state.db.get_subject(&subject_id).await {
        Ok(Some(s)) => s,
        _ => return Err(AppError::NotFound("Subject not found".to_string())),
    };

    if current_user.id != subject.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to update this chapter".to_string(),
        ));
    }

    let mut chapter = match state.db.get_chapter(&subject_id, &chapter_id).await {
        Ok(Some(c)) => c,
        _ => return Err(AppError::NotFound("Chapter not found".to_string())),
    };

    if let Some(title) = payload.title {
        chapter.title = title;
    }
    if let Some(start) = payload.page_start {
        chapter.page_start = start;
    }
    if let Some(end) = payload.page_end {
        chapter.page_end = end;
    }
    if let Some(order) = payload.order_index {
        chapter.order_index = order;
    }

    state
        .db
        .save_chapter(&chapter)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(Json(ChapterResponse::from(chapter)))
}

#[utoipa::path(
    delete,
    path = "/api/chapters/{subject_id}/{chapter_id}",
    params(
        ("subject_id" = String, Path, description = "Subject ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 204, description = "Successfully deleted chapter"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn delete_chapter(
    State(state): State<Arc<AppState>>,
    Path((subject_id, chapter_id)): Path<(String, String)>,
    AuthUser(current_user): AuthUser,
) -> Result<StatusCode, AppError> {
    let chapter = match state.db.get_chapter(&subject_id, &chapter_id).await {
        Ok(Some(c)) => c,
        _ => return Err(AppError::NotFound("Chapter not found".to_string())),
    };

    if current_user.id != chapter.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to delete this chapter".to_string(),
        ));
    }

    // Delete slides
    let slides = state
        .db
        .get_slides_by_chapter(&chapter_id)
        .await
        .unwrap_or_default();
    for slide in slides {
        let _ = state.db.delete_slide(&chapter_id, &slide.id).await;
    }

    state
        .db
        .delete_chapter(&subject_id, &chapter_id)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GenerativeResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ChapterEmbedResponse {
    pub embed_url: String,
    pub expires_in: u64,
    pub package_key: String,
    pub scene_count: i32,
}

const EMBED_TTL_SECS: u64 = 3600;

#[utoipa::path(
    post,
    path = "/api/chapters/{subject_id}/{chapter_id}/slides/generate",
    params(
        ("subject_id" = String, Path, description = "Subject ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 200, description = "Successfully triggered slide generation", body = GenerativeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn generate_slides(
    State(state): State<Arc<AppState>>,
    Path((subject_id, chapter_id)): Path<(String, String)>,
    AuthUser(current_user): AuthUser,
) -> Result<Json<GenerativeResponse>, AppError> {
    let mut chapter = load_owned_chapter(&state, &subject_id, &chapter_id, &current_user.id).await?;
    mark_chapter_processing(&state, &mut chapter).await;

    let job = state
        .bg_tasks
        .start_generate_slides(current_user.id.clone(), None, subject_id, chapter_id)
        .await
        .map_err(AppError::InternalServerError)?;

    chapter.job_id = Some(job.id.clone());
    let _ = state.db.save_chapter(&chapter).await;

    Ok(Json(GenerativeResponse {
        message: "Slide generation started".to_string(),
        job_id: Some(job.id),
    }))
}

#[utoipa::path(
    post,
    path = "/api/chapters/{subject_id}/{chapter_id}/generate",
    params(
        ("subject_id" = String, Path, description = "Subject ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 200, description = "Successfully triggered interactive chapter generation", body = GenerativeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn generate_chapter(
    State(state): State<Arc<AppState>>,
    Path((subject_id, chapter_id)): Path<(String, String)>,
    AuthUser(current_user): AuthUser,
) -> Result<Json<GenerativeResponse>, AppError> {
    let mut chapter = load_owned_chapter(&state, &subject_id, &chapter_id, &current_user.id).await?;
    mark_chapter_processing(&state, &mut chapter).await;

    let job = state
        .bg_tasks
        .start_generate_chapter(current_user.id.clone(), None, subject_id, chapter_id)
        .await
        .map_err(AppError::InternalServerError)?;

    chapter.job_id = Some(job.id.clone());
    let _ = state.db.save_chapter(&chapter).await;

    Ok(Json(GenerativeResponse {
        message: "Chapter generation started".to_string(),
        job_id: Some(job.id),
    }))
}

async fn load_owned_chapter(
    state: &AppState,
    subject_id: &str,
    chapter_id: &str,
    user_id: &str,
) -> Result<Chapter, AppError> {
    let chapter = match state.db.get_chapter(subject_id, chapter_id).await {
        Ok(Some(c)) => c,
        _ => return Err(AppError::NotFound("Chapter not found".to_string())),
    };
    if user_id != chapter.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to generate content for this chapter".to_string(),
        ));
    }
    Ok(chapter)
}

async fn mark_chapter_processing(state: &AppState, chapter: &mut Chapter) {
    chapter.processing_status = Some("processing".to_string());
    chapter.processed_slides = None;
    chapter.total_slides = None;
    chapter.updated_at = Utc::now();
    let _ = state.db.save_chapter(chapter).await;
}

#[utoipa::path(
    get,
    path = "/api/chapters/{subject_id}/{chapter_id}/embed",
    params(
        ("subject_id" = String, Path, description = "Subject ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 200, description = "Presigned URL for the isolated chapter iframe", body = ChapterEmbedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Chapter package not ready")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_chapter_embed(
    State(state): State<Arc<AppState>>,
    Path((subject_id, chapter_id)): Path<(String, String)>,
    AuthUser(current_user): AuthUser,
) -> Result<Json<ChapterEmbedResponse>, AppError> {
    let chapter = match state.db.get_chapter(&subject_id, &chapter_id).await {
        Ok(Some(c)) => c,
        _ => return Err(AppError::NotFound("Chapter not found".to_string())),
    };

    if current_user.id != chapter.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to view this chapter".to_string(),
        ));
    }

    if chapter.package_key.is_none() {
        return Err(AppError::NotFound("Chapter package not ready".to_string()));
    }

    let html_key = match state
        .bg_tasks
        .refresh_chapter_package(&chapter.user_id, &subject_id, &chapter_id, EMBED_TTL_SECS)
        .await
    {
        Ok(key) => key,
        Err(e) => {
            warn!(
                error = format!("{e:#}"),
                chapter_id, "Falling back to stored chapter HTML"
            );
            chapter
                .package_key
                .clone()
                .ok_or_else(|| AppError::NotFound("Chapter package not ready".to_string()))?
        }
    };

    let embed_url = state
        .storage
        .get_presigned_url(&html_key, EMBED_TTL_SECS)
        .await
        .map_err(AppError::InternalServerError)?;

    let scene_count = chapter.total_slides.unwrap_or(0);

    Ok(Json(ChapterEmbedResponse {
        embed_url,
        expires_in: EMBED_TTL_SECS,
        package_key: html_key,
        scene_count,
    }))
}

#[utoipa::path(
    get,
    path = "/api/chapters/{chapter_id}/slides",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 200, description = "Successfully fetched chapter slides", body = [SlideResponse]),
        (status = 404, description = "Not found")
    ),
    tag = "Chapters"
)]
pub async fn get_chapter_slides(
    State(state): State<Arc<AppState>>,
    Path(chapter_id): Path<String>,
) -> Result<Json<Vec<SlideResponse>>, AppError> {
    let slides = state
        .db
        .get_slides_by_chapter(&chapter_id)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    let mut response = Vec::new();

    for mut slide in slides {
        if let Some(voice_url) = &slide.voice_url
            && let Ok(presigned) = state.storage.get_presigned_url(voice_url, 3600).await
        {
            slide.voice_url = Some(presigned);
        }
        response.push(SlideResponse::from(slide));
    }

    Ok(Json(response))
}

#[utoipa::path(
    patch,
    path = "/api/chapters/{chapter_id}/slides/{slide_id}",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("slide_id" = String, Path, description = "Slide ID")
    ),
    request_body = SlideUpdate,
    responses(
        (status = 200, description = "Successfully updated slide", body = SlideResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn update_slide(
    State(state): State<Arc<AppState>>,
    Path((chapter_id, slide_id)): Path<(String, String)>,
    AuthUser(current_user): AuthUser,
    Json(payload): Json<SlideUpdate>,
) -> Result<Json<SlideResponse>, AppError> {
    let mut slide = match state.db.get_slide(&chapter_id, &slide_id).await {
        Ok(Some(s)) => s,
        _ => return Err(AppError::NotFound("Slide not found".to_string())),
    };

    if current_user.id != slide.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to update this slide".to_string(),
        ));
    }

    if let Some(title) = payload.title {
        slide.title = title;
    }
    if let Some(points) = payload.points {
        slide.points = points;
    }
    if let Some(explanation) = payload.explanation {
        slide.explanation = explanation;
    }
    if let Some(order) = payload.order_index {
        slide.order_index = order;
    }
    if let Some(voice_url) = payload.voice_url {
        slide.voice_url = Some(voice_url);
    }

    state
        .db
        .save_slide(&slide)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(Json(SlideResponse::from(slide)))
}

#[utoipa::path(
    delete,
    path = "/api/chapters/{chapter_id}/slides/{slide_id}",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("slide_id" = String, Path, description = "Slide ID")
    ),
    responses(
        (status = 204, description = "Successfully deleted slide"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Chapters",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn delete_slide(
    State(state): State<Arc<AppState>>,
    Path((chapter_id, slide_id)): Path<(String, String)>,
    AuthUser(current_user): AuthUser,
) -> Result<StatusCode, AppError> {
    let slide = match state.db.get_slide(&chapter_id, &slide_id).await {
        Ok(Some(s)) => s,
        _ => return Err(AppError::NotFound("Slide not found".to_string())),
    };

    if current_user.id != slide.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to delete this slide".to_string(),
        ));
    }

    state
        .db
        .delete_slide(&chapter_id, &slide_id)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(StatusCode::NO_CONTENT)
}
