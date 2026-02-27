use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::api::extractors::{AuthUser, OptionalAuthUser};
use crate::error::AppError;
use crate::models::{SubjectResponse, UserResponse};

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

pub fn router() -> Router<Arc<AppState>> {
    use axum::routing::{delete, patch};
    Router::new()
        .route("/me", get(get_myprofile))
        .route("/me", patch(update_user))
        .route("/me", delete(delete_user))
        .route("/{user_id}", get(get_user))
        .route("/{user_id}/subjects", get(get_user_subjects))
}

#[utoipa::path(
    get,
    path = "/api/users/me",
    responses(
        (status = 200, description = "Successfully fetched profile", body = UserResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Users",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_myprofile(
    State(state): State<Arc<AppState>>,
    AuthUser(mut current_user): AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    if let Some(dp) = &current_user.dp {
        if let Ok(url) = state.storage.get_presigned_url(dp, 3600).await {
            current_user.dp = Some(url);
        }
    }
    Ok(Json(UserResponse::from(current_user)))
}

#[utoipa::path(
    get,
    path = "/api/users/{user_id}",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Successfully fetched user", body = UserResponse),
        (status = 404, description = "Not found")
    ),
    tag = "Users"
)]
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<UserResponse>, AppError> {
    let mut user = match state.db.get_user_by_id(&user_id).await {
        Ok(Some(u)) if u.is_active => u,
        _ => return Err(AppError::NotFound("User not found".to_string())),
    };

    if let Some(dp) = &user.dp {
        if let Ok(url) = state.storage.get_presigned_url(dp, 3600).await {
            user.dp = Some(url);
        }
    }

    Ok(Json(UserResponse::from(user)))
}

#[utoipa::path(
    get,
    path = "/api/users/{user_id}/subjects",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Successfully fetched user subjects", body = [SubjectResponse]),
        (status = 404, description = "User not found")
    ),
    tag = "Users",
    security(
        (),
        ("bearerAuth" = [])
    )
)]
pub async fn get_user_subjects(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Query(pagination): Query<Pagination>,
    OptionalAuthUser(current_user): OptionalAuthUser,
) -> Result<Json<Vec<SubjectResponse>>, AppError> {
    // Check if user exists
    let user = match state.db.get_user_by_id(&user_id).await {
        Ok(Some(u)) => u,
        _ => return Err(AppError::NotFound("User not found".to_string())),
    };

    let public_only = match current_user {
        Some(u) if u.id == user.id => false, // Can see everything
        _ => true,                           // Only public
    };

    let mut subjects = state
        .db
        .get_user_subjects_by_id(&user.id, public_only)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

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
    patch,
    path = "/api/users/me",
    request_body(content = String, description = "Can upload `user_update` JSON and `dp` file via multipart/form-data", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Successfully updated profile", body = UserResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Users",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    AuthUser(mut current_user): AuthUser,
    mut multipart: Multipart,
) -> Result<Json<UserResponse>, AppError> {
    let mut update_data = None;
    let mut file_content = None;
    let mut file_ext = None;

    while let Some(field) = multipart.next_field().await.map_err(|e: axum::extract::multipart::MultipartError| AppError::BadRequest(format!("Multipart error: {}", e)))? {
        let name = field.name().unwrap_or("").to_string();

        if name == "user_update" {
            let data = field.text().await.map_err(|e: axum::extract::multipart::MultipartError| AppError::BadRequest(format!("Error reading user_update: {}", e)))?;
            update_data = serde_json::from_str::<crate::models::UserUpdate>(&data).ok();
        } else if name == "dp" {
            let filename = field.file_name().unwrap_or("").to_string();
            if filename.to_lowercase().ends_with(".jpg") {
                file_ext = Some("jpg".to_string());
            } else if filename.to_lowercase().ends_with(".png") {
                file_ext = Some("png".to_string());
            } else if filename.to_lowercase().ends_with(".jpeg") {
                file_ext = Some("jpeg".to_string());
            } else {
                return Err(AppError::BadRequest("Invalid image format. Use jpg, png or jpeg".to_string()));
            }

            let data = field.bytes().await.map_err(|e: axum::extract::multipart::MultipartError| AppError::BadRequest(format!("Error reading dp file: {}", e)))?;
            if data.len() > 5 * 1024 * 1024 {
                return Err(AppError::BadRequest("File size too large (max 5MB)".to_string()));
            }
            file_content = Some(data.to_vec());
        }
    }

    if let (Some(content), Some(ext)) = (file_content, file_ext) {
        let avatar_key = format!("avatars/{}_{}.{}", current_user.id, Uuid::new_v4(), ext);
        if state
            .storage
            .upload_file(&avatar_key, content)
            .await
            .is_ok()
        {
            current_user.dp = Some(avatar_key);
        }
    }

    if let Some(update) = update_data {
        if let Some(name) = update.full_name {
            current_user.full_name = name;
        }
    }

    state
        .db
        .save_user(&current_user)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    if let Some(dp) = &current_user.dp {
        if let Ok(url) = state.storage.get_presigned_url(dp, 3600).await {
            current_user.dp = Some(url);
        }
    }

    Ok(Json(UserResponse::from(current_user)))
}

#[utoipa::path(
    delete,
    path = "/api/users/me",
    responses(
        (status = 204, description = "Successfully deleted user"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Users",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    AuthUser(mut current_user): AuthUser,
) -> Result<StatusCode, AppError> {
    current_user.is_active = false;
    state
        .db
        .save_user(&current_user)
        .await
        .map_err(|e: anyhow::Error| AppError::InternalServerError(e))?;

    Ok(StatusCode::NO_CONTENT)
}
