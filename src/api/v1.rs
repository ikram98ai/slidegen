//! Service-to-service API used by sibling backends (khaneducation, knoio, …).
//! Authenticate with `X-Api-Key`.

use axum::{
    Json, Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::AppState;
use crate::api::chapters::{ChapterEmbedResponse, GenerativeResponse};
use crate::api::extractors::ServiceAuth;
use crate::error::AppError;
use crate::models::{
    ChapterResponse, IngestBookRequest, IngestBookResponse, JobResponse, SceneSpec, Subject,
    SubjectDetailResponse, SubjectResponse, SubjectType,
};
use crate::services::retrieval::{
    AskRequest, AskResponse, SearchHit, SearchRequest, SearchResponse,
};

const EMBED_TTL_SECS: u64 = 3600;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/books", post(upload_book))
        .route("/books/ingest", post(ingest_book))
        .route("/jobs/{job_id}", get(get_job))
        .route("/books/{book_id}", get(get_book))
        .route("/books/{book_id}/chapters", get(list_chapters))
        .route("/books/{book_id}/chapters/{chapter_id}", get(get_chapter))
        .route(
            "/books/{book_id}/chapters/{chapter_id}/embed",
            get(get_chapter_embed),
        )
        .route(
            "/books/{book_id}/chapters/{chapter_id}/generate",
            post(generate_chapter),
        )
        .route("/books/{book_id}/search", post(search_book))
        .route(
            "/books/{book_id}/chapters/{chapter_id}/ask",
            post(ask_chapter),
        )
        .route(
            "/books/{book_id}/chapters/{chapter_id}/scenes/{scene_id}",
            get(get_scene),
        )
        .route(
            "/books/{book_id}/paragraphs/{paragraph_id}",
            get(get_paragraph),
        )
}

async fn create_book_and_start(
    state: &AppState,
    tenant_id: &str,
    title: String,
    subject_type: SubjectType,
    is_public: bool,
    file_path: String,
) -> Result<IngestBookResponse, AppError> {
    let user_id = format!("tenant:{tenant_id}");
    let subject_id = Uuid::new_v4().to_string();

    let subject = Subject {
        id: subject_id.clone(),
        user_id: user_id.clone(),
        title,
        file_path: file_path.clone(),
        is_public,
        r#type: subject_type,
        processing_status: "processing".to_string(),
        processed_pages: None,
        total_pages: None,
        job_id: None,
        processing_stage: Some("queued".to_string()),
        extract_prefix: None,
        page_offset: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    state
        .db
        .save_subject(&subject)
        .await
        .map_err(AppError::InternalServerError)?;

    let job = state
        .bg_tasks
        .start_process_subject(user_id, Some(tenant_id.to_string()), subject_id, file_path)
        .await
        .map_err(AppError::InternalServerError)?;

    let mut subject = state
        .db
        .get_subject(&subject.id)
        .await
        .map_err(AppError::InternalServerError)?
        .unwrap_or(subject);

    subject.job_id = Some(job.id.clone());
    state
        .db
        .save_subject(&subject)
        .await
        .map_err(AppError::InternalServerError)?;
    Ok(IngestBookResponse {
        book: SubjectResponse::from(subject),
        job: JobResponse::from(job),
    })
}

#[utoipa::path(
    post,
    path = "/api/v1/books",
    request_body(content = String, description = "multipart fields: title, type, file, optional is_public", content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Book accepted for processing", body = IngestBookResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Service",
    security(
        ("apiKeyAuth" = [])
    )
)]
pub async fn upload_book(
    State(state): State<Arc<AppState>>,
    service: ServiceAuth,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<IngestBookResponse>), AppError> {
    let mut title: Option<String> = None;
    let mut subject_type: Option<SubjectType> = None;
    let mut is_public = false;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::BadRequest(format!("Error reading title: {e}")))?,
                );
            }
            "type" => {
                let t = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Error reading type: {e}")))?;
                subject_type = Some(match t.to_lowercase().as_str() {
                    "book" => SubjectType::Book,
                    "report" => SubjectType::Report,
                    _ => return Err(AppError::BadRequest("Invalid subject type".to_string())),
                });
            }
            "is_public" => {
                let val = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Error reading is_public: {e}")))?;
                is_public = val.parse().unwrap_or(false);
            }
            "file" => {
                file_name = Some(field.file_name().unwrap_or("file").to_string());
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| AppError::BadRequest(format!("Error reading file: {e}")))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let (title, subject_type, file_data, file_name) =
        match (title, subject_type, file_data, file_name) {
            (Some(t), Some(st), Some(fd), Some(fnm)) => (t, st, fd, fnm),
            _ => {
                return Err(AppError::BadRequest(
                    "Missing required fields (title, type, and file)".to_string(),
                ));
            }
        };

    let subject_id = Uuid::new_v4().to_string();
    let extension = file_name.split('.').next_back().unwrap_or("bin");
    let file_path = format!(
        "subjects/{}/{}.{}",
        service.user_id(),
        subject_id,
        extension
    );

    state
        .storage
        .upload_file(&file_path, file_data)
        .await
        .map_err(AppError::InternalServerError)?;

    // Re-create with the reserved id so the S3 key matches the subject row.
    let user_id = service.user_id();
    let tenant_id = service.tenant_id.clone();

    let subject = Subject {
        id: subject_id.clone(),
        user_id: user_id.clone(),
        title,
        file_path: file_path.clone(),
        is_public,
        r#type: subject_type,
        processing_status: "processing".to_string(),
        processed_pages: None,
        total_pages: None,
        job_id: None,
        processing_stage: Some("queued".to_string()),
        extract_prefix: None,
        page_offset: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    state
        .db
        .save_subject(&subject)
        .await
        .map_err(AppError::InternalServerError)?;

    let job = state
        .bg_tasks
        .start_process_subject(user_id, Some(tenant_id), subject_id, file_path)
        .await
        .map_err(AppError::InternalServerError)?;

    let mut subject = state
        .db
        .get_subject(&subject.id)
        .await
        .map_err(AppError::InternalServerError)?
        .unwrap_or(subject);
    subject.job_id = Some(job.id.clone());
    state
        .db
        .save_subject(&subject)
        .await
        .map_err(AppError::InternalServerError)?;

    Ok((
        StatusCode::CREATED,
        Json(IngestBookResponse {
            book: SubjectResponse::from(subject),
            job: JobResponse::from(job),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/books/ingest",
    request_body = IngestBookRequest,
    responses(
        (status = 201, description = "Book accepted for processing", body = IngestBookResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Service",
    security(
        ("apiKeyAuth" = [])
    )
)]
pub async fn ingest_book(
    State(state): State<Arc<AppState>>,
    service: ServiceAuth,
    Json(payload): Json<IngestBookRequest>,
) -> Result<(StatusCode, Json<IngestBookResponse>), AppError> {
    if payload.file_s3path.trim().is_empty() {
        return Err(AppError::BadRequest("file_s3path is required".to_string()));
    }

    let response = create_book_and_start(
        &state,
        &service.tenant_id,
        payload.title,
        payload.r#type,
        payload.is_public,
        payload.file_s3path,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(response)))
}

fn tenant_owns(service: &ServiceAuth, subject: &Subject) -> bool {
    subject.user_id == service.user_id()
}

async fn load_owned_book(
    state: &AppState,
    service: &ServiceAuth,
    book_id: &str,
) -> Result<Subject, AppError> {
    let subject = state
        .db
        .get_subject(book_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Book not found".to_string()))?;
    if !tenant_owns(service, &subject) {
        return Err(AppError::Forbidden(
            "You do not have permission to access this book".to_string(),
        ));
    }
    Ok(subject)
}

#[utoipa::path(
    get,
    path = "/api/v1/jobs/{job_id}",
    params(("job_id" = String, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job progress", body = JobResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
    service: ServiceAuth,
) -> Result<Json<JobResponse>, AppError> {
    let job = state
        .db
        .get_job(&job_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Job not found".to_string()))?;
    if job.user_id != service.user_id()
        && job.tenant_id.as_deref() != Some(service.tenant_id.as_str())
    {
        return Err(AppError::Forbidden(
            "You do not have permission to view this job".to_string(),
        ));
    }
    Ok(Json(JobResponse::from(job)))
}

#[utoipa::path(
    get,
    path = "/api/v1/books/{book_id}",
    params(("book_id" = String, Path, description = "Book / subject ID")),
    responses(
        (status = 200, description = "Book", body = SubjectResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn get_book(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<String>,
    service: ServiceAuth,
) -> Result<Json<SubjectResponse>, AppError> {
    let subject = load_owned_book(&state, &service, &book_id).await?;
    Ok(Json(SubjectResponse::from(subject)))
}

#[utoipa::path(
    get,
    path = "/api/v1/books/{book_id}/chapters",
    params(("book_id" = String, Path, description = "Book / subject ID")),
    responses(
        (status = 200, description = "Book with chapters", body = SubjectDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn list_chapters(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<String>,
    service: ServiceAuth,
) -> Result<Json<SubjectDetailResponse>, AppError> {
    let subject = load_owned_book(&state, &service, &book_id).await?;
    let chapters = state
        .db
        .get_chapters_by_subject(&subject.id)
        .await
        .map_err(AppError::InternalServerError)?;
    Ok(Json(SubjectDetailResponse {
        subject: SubjectResponse::from(subject),
        chapters: chapters.into_iter().map(ChapterResponse::from).collect(),
    }))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ServiceChapterResponse {
    #[serde(flatten)]
    pub chapter: ChapterResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/books/{book_id}/chapters/{chapter_id}",
    params(
        ("book_id" = String, Path, description = "Book ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 200, description = "Chapter plus optional signed embed URL", body = ServiceChapterResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn get_chapter(
    State(state): State<Arc<AppState>>,
    Path((book_id, chapter_id)): Path<(String, String)>,
    service: ServiceAuth,
) -> Result<Json<ServiceChapterResponse>, AppError> {
    let subject = load_owned_book(&state, &service, &book_id).await?;
    let chapter = state
        .db
        .get_chapter(&subject.id, &chapter_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Chapter not found".to_string()))?;

    let (embed_url, expires_in) = if chapter.package_key.is_some() {
        match signed_embed(&state, &chapter.user_id, &subject.id, &chapter.id).await {
            Ok(url) => (Some(url), Some(EMBED_TTL_SECS)),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    Ok(Json(ServiceChapterResponse {
        chapter: ChapterResponse::from(chapter),
        embed_url,
        expires_in,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/books/{book_id}/chapters/{chapter_id}/embed",
    params(
        ("book_id" = String, Path, description = "Book ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 200, description = "Presigned iframe URL", body = ChapterEmbedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Chapter package not ready")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn get_chapter_embed(
    State(state): State<Arc<AppState>>,
    Path((book_id, chapter_id)): Path<(String, String)>,
    service: ServiceAuth,
) -> Result<Json<ChapterEmbedResponse>, AppError> {
    let subject = load_owned_book(&state, &service, &book_id).await?;
    let chapter = state
        .db
        .get_chapter(&subject.id, &chapter_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Chapter not found".to_string()))?;
    if chapter.package_key.is_none() {
        return Err(AppError::NotFound("Chapter package not ready".to_string()));
    }
    let html_key = match state
        .bg_tasks
        .refresh_chapter_package(&chapter.user_id, &subject.id, &chapter.id, EMBED_TTL_SECS)
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
    Ok(Json(ChapterEmbedResponse {
        embed_url,
        expires_in: EMBED_TTL_SECS,
        package_key: html_key,
        scene_count: chapter.total_slides.unwrap_or(0),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/books/{book_id}/chapters/{chapter_id}/generate",
    params(
        ("book_id" = String, Path, description = "Book ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    responses(
        (status = 200, description = "Chapter generation started", body = GenerativeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn generate_chapter(
    State(state): State<Arc<AppState>>,
    Path((book_id, chapter_id)): Path<(String, String)>,
    service: ServiceAuth,
) -> Result<Json<GenerativeResponse>, AppError> {
    let subject = load_owned_book(&state, &service, &book_id).await?;
    let mut chapter = state
        .db
        .get_chapter(&subject.id, &chapter_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Chapter not found".to_string()))?;
    chapter.processing_status = Some("processing".to_string());
    chapter.processed_slides = None;
    chapter.total_slides = None;
    chapter.updated_at = Utc::now();
    let _ = state.db.save_chapter(&chapter).await;

    let job = state
        .bg_tasks
        .start_generate_chapter(
            service.user_id(),
            Some(service.tenant_id.clone()),
            subject.id,
            chapter_id,
        )
        .await
        .map_err(AppError::InternalServerError)?;

    chapter.job_id = Some(job.id.clone());
    let _ = state.db.save_chapter(&chapter).await;

    Ok(Json(GenerativeResponse {
        message: "Chapter generation started".to_string(),
        job_id: Some(job.id),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/books/{book_id}/search",
    params(("book_id" = String, Path, description = "Book ID")),
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Cited passages", body = SearchResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn search_book(
    State(state): State<Arc<AppState>>,
    Path(book_id): Path<String>,
    service: ServiceAuth,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, AppError> {
    if payload.query.trim().is_empty() {
        return Err(AppError::BadRequest("query is required".to_string()));
    }
    let subject = load_owned_book(&state, &service, &book_id).await?;
    let chapter = if let Some(chapter_id) = payload.chapter_id.as_deref() {
        Some(
            state
                .db
                .get_chapter(&subject.id, chapter_id)
                .await
                .map_err(AppError::InternalServerError)?
                .ok_or_else(|| AppError::NotFound("Chapter not found".to_string()))?,
        )
    } else {
        None
    };
    let hits = state
        .retrieval
        .search(
            &subject,
            chapter.as_ref(),
            payload.query.trim(),
            payload.limit.unwrap_or(8),
        )
        .await
        .map_err(AppError::InternalServerError)?;
    Ok(Json(SearchResponse { hits }))
}

#[utoipa::path(
    post,
    path = "/api/v1/books/{book_id}/chapters/{chapter_id}/ask",
    params(
        ("book_id" = String, Path, description = "Book ID"),
        ("chapter_id" = String, Path, description = "Chapter ID")
    ),
    request_body = AskRequest,
    responses(
        (status = 200, description = "Grounded answer with citations", body = AskResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn ask_chapter(
    State(state): State<Arc<AppState>>,
    Path((book_id, chapter_id)): Path<(String, String)>,
    service: ServiceAuth,
    Json(payload): Json<AskRequest>,
) -> Result<Json<AskResponse>, AppError> {
    if payload.question.trim().is_empty() {
        return Err(AppError::BadRequest("question is required".to_string()));
    }
    let subject = load_owned_book(&state, &service, &book_id).await?;
    let chapter = state
        .db
        .get_chapter(&subject.id, &chapter_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Chapter not found".to_string()))?;
    let response = state
        .retrieval
        .ask(&subject, Some(&chapter), payload.question.trim())
        .await
        .map_err(AppError::InternalServerError)?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v1/books/{book_id}/chapters/{chapter_id}/scenes/{scene_id}",
    params(
        ("book_id" = String, Path, description = "Book ID"),
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("scene_id" = String, Path, description = "Scene ID")
    ),
    responses(
        (status = 200, description = "Scene spec", body = SceneSpec),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn get_scene(
    State(state): State<Arc<AppState>>,
    Path((book_id, chapter_id, scene_id)): Path<(String, String, String)>,
    service: ServiceAuth,
) -> Result<Json<SceneSpec>, AppError> {
    let subject = load_owned_book(&state, &service, &book_id).await?;
    state
        .retrieval
        .get_scene(&subject, &chapter_id, &scene_id)
        .await
        .map_err(AppError::InternalServerError)?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Scene not found".to_string()))
}

#[utoipa::path(
    get,
    path = "/api/v1/books/{book_id}/paragraphs/{paragraph_id}",
    params(
        ("book_id" = String, Path, description = "Book ID"),
        ("paragraph_id" = String, Path, description = "Paragraph ID, e.g. p12-2")
    ),
    responses(
        (status = 200, description = "Cited paragraph", body = SearchHit),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Service",
    security(("apiKeyAuth" = []))
)]
pub async fn get_paragraph(
    State(state): State<Arc<AppState>>,
    Path((book_id, paragraph_id)): Path<(String, String)>,
    service: ServiceAuth,
) -> Result<Json<SearchHit>, AppError> {
    let subject = load_owned_book(&state, &service, &book_id).await?;
    state
        .retrieval
        .get_citation(&subject, &paragraph_id)
        .await
        .map_err(AppError::InternalServerError)?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Paragraph not found".to_string()))
}

async fn signed_embed(
    state: &AppState,
    user_id: &str,
    subject_id: &str,
    chapter_id: &str,
) -> Result<String, AppError> {
    let html_key = state
        .bg_tasks
        .refresh_chapter_package(user_id, subject_id, chapter_id, EMBED_TTL_SECS)
        .await
        .map_err(AppError::InternalServerError)?;
    state
        .storage
        .get_presigned_url(&html_key, EMBED_TTL_SECS)
        .await
        .map_err(AppError::InternalServerError)
}
