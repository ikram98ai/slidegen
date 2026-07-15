use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::api::extractors::{AuthUser, OptionalAuthUser};
use crate::error::AppError;
use crate::models::{
    ChapterResponse, Subject, SubjectCreate, SubjectDetailResponse, SubjectResponse, SubjectType,
    SubjectUpdate,
};
use crate::services::bg_tasks::Job;
use chrono::Utc;
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    use axum::routing::{delete, patch, post};
    Router::new()
        .route("/", get(list_subjects))
        .route("/", post(upload_subject))
        .route("/{subject_id}", get(get_subject))
        .route("/{subject_id}", patch(update_subject))
        .route("/{subject_id}", delete(delete_subject))
        .route("/{subject_id}/chapters", get(get_subject_chapters))
        .route("/{subject_id}/reprocess", post(reprocess_subject))
}

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "default_skip")]
    pub skip: i32,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_skip() -> i32 {
    0
}
fn default_limit() -> i32 {
    100
}

#[utoipa::path(
    get,
    path = "/api/subjects",
    responses(
        (status = 200, description = "Successfully fetched subjects", body = [SubjectResponse])
    ),
    tag = "Subjects",
    security(
        (),
        ("bearerAuth" = [])
    )
)]
pub async fn list_subjects(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
    OptionalAuthUser(current_user): OptionalAuthUser,
) -> Result<Json<Vec<SubjectResponse>>, AppError> {
    let mut subjects = if current_user.is_some() {
        state
            .db
            .list_subjects(100)
            .await
            .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?
    } else {
        state
            .db
            .list_public_subjects(100)
            .await
            .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?
    };

    subjects.sort_by_key(|s| std::cmp::Reverse(s.created_at));

    let start = pagination.skip as usize;
    let end = (pagination.skip + pagination.limit) as usize;

    let end = std::cmp::min(end, subjects.len());
    let paginated = if start < end {
        subjects[start..end].to_vec()
    } else {
        vec![]
    };

    let response = paginated.into_iter().map(SubjectResponse::from).collect();

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/subjects",
    request_body(content = String, description = "Can upload `subject_create` JSON or individual fields (title, type, is_public) and `file` via multipart/form-data", content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Successfully created subject", body = SubjectResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad Request")
    ),
    tag = "Subjects",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn upload_subject(
    State(state): State<Arc<AppState>>,
    AuthUser(current_user): AuthUser,
    mut multipart: Multipart,
) -> Result<Json<SubjectResponse>, AppError> {
    let mut title: Option<String> = None;
    let mut subject_type: Option<SubjectType> = None;
    let mut is_public: bool = false;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) =
        multipart
            .next_field()
            .await
            .map_err(|e: axum::extract::multipart::MultipartError| {
                AppError::BadRequest(format!("Multipart error: {}", e))
            })?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "subject_create" => {
                let data = field.bytes().await.map_err(
                    |e: axum::extract::multipart::MultipartError| {
                        AppError::BadRequest(format!("Error reading subject_create: {}", e))
                    },
                )?;
                let sc: SubjectCreate =
                    serde_json::from_slice(&data).map_err(|e: serde_json::Error| {
                        AppError::BadRequest(format!("Invalid subject_create JSON: {}", e))
                    })?;
                title = Some(sc.title);
                subject_type = Some(sc.r#type);
                is_public = sc.is_public;
            }
            "title" => {
                title = Some(field.text().await.map_err(
                    |e: axum::extract::multipart::MultipartError| {
                        AppError::BadRequest(format!("Error reading title: {}", e))
                    },
                )?);
            }
            "type" => {
                let t =
                    field
                        .text()
                        .await
                        .map_err(|e: axum::extract::multipart::MultipartError| {
                            AppError::BadRequest(format!("Error reading type: {}", e))
                        })?;
                subject_type = Some(match t.to_lowercase().as_str() {
                    "book" => SubjectType::Book,
                    "report" => SubjectType::Report,
                    _ => return Err(AppError::BadRequest("Invalid subject type".to_string())),
                });
            }
            "is_public" => {
                let val =
                    field
                        .text()
                        .await
                        .map_err(|e: axum::extract::multipart::MultipartError| {
                            AppError::BadRequest(format!("Error reading is_public: {}", e))
                        })?;
                is_public = val.parse().unwrap_or(false);
            }
            "file" => {
                file_name = Some(field.file_name().unwrap_or("file").to_string());
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e: axum::extract::multipart::MultipartError| {
                            AppError::BadRequest(format!("Error reading file: {}", e))
                        })?
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
    let file_path = format!("subjects/{}/{}.{}", current_user.id, subject_id, extension);

    // 1. Upload to S3
    state
        .storage
        .upload_file(&file_path, file_data)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    // 2. Create Subject record in DB
    let subject = Subject {
        id: subject_id.clone(),
        user_id: current_user.id.clone(),
        title,
        file_path: file_path.clone(),
        is_public,
        r#type: subject_type,
        processing_status: "processing".to_string(),
        processed_pages: None,
        total_pages: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    state
        .db
        .save_subject(&subject)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    // 3. Trigger background processing (SQS on Lambda, in-process locally)
    state
        .bg_tasks
        .dispatch(Job::ProcessSubject {
            user_id: current_user.id.clone(),
            subject_id: subject_id.clone(),
            file_s3path: file_path.clone(),
        })
        .await
        .map_err(AppError::InternalServerError)?;

    Ok(Json(SubjectResponse::from(subject)))
}

#[utoipa::path(
    post,
    path = "/api/subjects/{subject_id}/reprocess",
    params(
        ("subject_id" = String, Path, description = "Subject ID")
    ),
    responses(
        (status = 200, description = "Reprocessing started", body = SubjectResponse),
        (status = 400, description = "Subject is already being processed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Subjects",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn reprocess_subject(
    State(state): State<Arc<AppState>>,
    Path(subject_id): Path<String>,
    AuthUser(current_user): AuthUser,
) -> Result<Json<SubjectResponse>, AppError> {
    let mut subject = state
        .db
        .get_subject(&subject_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Subject not found".to_string()))?;

    if current_user.id != subject.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to reprocess this subject".to_string(),
        ));
    }

    if subject.processing_status == "processing" {
        return Err(AppError::BadRequest(
            "Subject is already being processed".to_string(),
        ));
    }

    // Remove chapters (and their slides) left over from the previous run so
    // reprocessing doesn't create duplicates.
    let chapters = state
        .db
        .get_chapters_by_subject(&subject_id)
        .await
        .unwrap_or_default();
    for chapter in chapters {
        let slides = state
            .db
            .get_slides_by_chapter(&chapter.id)
            .await
            .unwrap_or_default();
        for slide in slides {
            let _ = state.db.delete_slide(&chapter.id, &slide.id).await;
        }
        let _ = state.db.delete_chapter(&subject_id, &chapter.id).await;
    }

    subject.processing_status = "processing".to_string();
    subject.processed_pages = None;
    subject.total_pages = None;
    subject.updated_at = Utc::now();
    state
        .db
        .save_subject(&subject)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    state
        .bg_tasks
        .dispatch(Job::ProcessSubject {
            user_id: current_user.id.clone(),
            subject_id: subject.id.clone(),
            file_s3path: subject.file_path.clone(),
        })
        .await
        .map_err(AppError::InternalServerError)?;

    Ok(Json(SubjectResponse::from(subject)))
}

#[utoipa::path(
    get,
    path = "/api/subjects/{subject_id}",
    params(
        ("subject_id" = String, Path, description = "Subject ID")
    ),
    responses(
        (status = 200, description = "Successfully fetched subject", body = SubjectResponse),
        (status = 404, description = "Not found")
    ),
    tag = "Subjects"
)]
pub async fn get_subject(
    State(state): State<Arc<AppState>>,
    Path(subject_id): Path<String>,
) -> Result<Json<SubjectResponse>, AppError> {
    let subject = state
        .db
        .get_subject(&subject_id)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    match subject {
        Some(s) => Ok(Json(SubjectResponse::from(s))),
        None => Err(AppError::NotFound("Subject not found".to_string())),
    }
}

#[utoipa::path(
    get,
    path = "/api/subjects/{subject_id}/chapters",
    params(
        ("subject_id" = String, Path, description = "Subject ID")
    ),
    responses(
        (status = 200, description = "Successfully fetched subject with its chapters", body = SubjectDetailResponse),
        (status = 404, description = "Not found")
    ),
    tag = "Subjects"
)]
pub async fn get_subject_chapters(
    State(state): State<Arc<AppState>>,
    Path(subject_id): Path<String>,
) -> Result<Json<SubjectDetailResponse>, AppError> {
    let subject = state
        .db
        .get_subject(&subject_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Subject not found".to_string()))?;

    let mut chapters = state
        .db
        .get_chapters_by_subject(&subject.id)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    chapters.sort_by_key(|c| c.order_index);

    Ok(Json(SubjectDetailResponse {
        subject: SubjectResponse::from(subject),
        chapters: chapters.into_iter().map(ChapterResponse::from).collect(),
    }))
}

#[utoipa::path(
    patch,
    path = "/api/subjects/{subject_id}",
    params(
        ("subject_id" = String, Path, description = "Subject ID")
    ),
    request_body = SubjectUpdate,
    responses(
        (status = 200, description = "Successfully updated subject", body = SubjectResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Subjects",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn update_subject(
    State(state): State<Arc<AppState>>,
    Path(subject_id): Path<String>,
    AuthUser(current_user): AuthUser,
    Json(payload): Json<crate::models::SubjectUpdate>,
) -> Result<Json<SubjectResponse>, AppError> {
    let mut subject = state
        .db
        .get_subject(&subject_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Subject not found".to_string()))?;

    if current_user.id != subject.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to update this subject".to_string(),
        ));
    }

    if let Some(title) = payload.title {
        subject.title = title;
    }
    if let Some(is_public) = payload.is_public {
        subject.is_public = is_public;
    }

    state
        .db
        .save_subject(&subject)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(Json(SubjectResponse::from(subject)))
}

#[utoipa::path(
    delete,
    path = "/api/subjects/{subject_id}",
    params(
        ("subject_id" = String, Path, description = "Subject ID")
    ),
    responses(
        (status = 204, description = "Successfully deleted subject"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Subjects",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn delete_subject(
    State(state): State<Arc<AppState>>,
    Path(subject_id): Path<String>,
    AuthUser(current_user): AuthUser,
) -> Result<StatusCode, AppError> {
    let subject = state
        .db
        .get_subject(&subject_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Subject not found".to_string()))?;

    if current_user.id != subject.user_id {
        return Err(AppError::Forbidden(
            "You do not have permission to delete this subject".to_string(),
        ));
    }

    // Must delete chapters and slides
    let chapters = state
        .db
        .get_chapters_by_subject(&subject_id)
        .await
        .unwrap_or_default();
    for chapter in chapters {
        let slides = state
            .db
            .get_slides_by_chapter(&chapter.id)
            .await
            .unwrap_or_default();
        for slide in slides {
            let _ = state.db.delete_slide(&chapter.id, &slide.id).await;
        }
        let _ = state.db.delete_chapter(&subject_id, &chapter.id).await;
    }

    state
        .db
        .delete_subject(&subject_id)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(StatusCode::NO_CONTENT)
}
