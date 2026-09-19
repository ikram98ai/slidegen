//! Service-to-service API used by sibling backends (khaneducation, knoio, …).
//! Authenticate with `X-Api-Key`.

use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::post,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::api::extractors::ServiceAuth;
use crate::error::AppError;
use crate::models::{
    IngestBookRequest, IngestBookResponse, JobResponse, Subject, SubjectResponse, SubjectType,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/books", post(upload_book))
        .route("/books/ingest", post(ingest_book))
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
