use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::error::AppError;
use crate::models::{User, UserCreate, UserResponse};
use crate::services::auth::{
    create_access_token, create_refresh_token, get_password_hash, verify_password, verify_token,
};

#[derive(Serialize, utoipa::ToSchema)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenReq {
    pub refresh_token: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/token", post(login))
        .route("/refresh", post(refresh_token))
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = UserCreate,
    responses(
        (status = 201, description = "Successfully registered user", body = UserResponse),
        (status = 400, description = "Bad request")
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserCreate>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    // Check if user exists
    if let Ok(Some(_)) = state.db.get_user_by_email(&payload.email).await {
        return Err(AppError::BadRequest("User with this email already exists".to_string()));
    }

    let hashed_password =
        get_password_hash(&payload.password).map_err(|e| AppError::InternalServerError(e.into()))?;

    let db_user = User {
        id: Uuid::new_v4().to_string(),
        full_name: payload.full_name,
        email: payload.email,
        hashed_password,
        dp: None,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    state
        .db
        .save_user(&db_user)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(db_user))))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginReq {
    pub email: String, 
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/token",
    request_body = LoginReq,
    responses(
        (status = 200, description = "Successfully logged in", body = Token),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginReq>,
) -> Result<Json<Token>, AppError> {
    let user = match state.db.get_user_by_email(&payload.email).await {
        Ok(Some(u)) => u,
        _ => return Err(AppError::Unauthorized("Invalid email or password".to_string())),
    };

    if !verify_password(&payload.password, &user.hashed_password) {
        return Err(AppError::Unauthorized("Invalid email or password".to_string()));
    }

    if !user.is_active {
        return Err(AppError::Forbidden("Your account is deactivated".to_string()));
    }

    let access_token = create_access_token(&user.id, &state.settings)
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    let refresh_token = create_refresh_token(&user.id, &state.settings)
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(Token {
        access_token,
        refresh_token,
        token_type: "bearer".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshTokenReq,
    responses(
        (status = 200, description = "Refreshed access token successfully", body = Token),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Auth"
)]
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshTokenReq>,
) -> Result<Json<Token>, AppError> {
    let claims = match verify_token(&payload.refresh_token, &state.settings) {
        Ok(c) => c,
        Err(_) => return Err(AppError::Unauthorized("Invalid refresh token".to_string())),
    };

    if claims.r#type != "refresh" {
        return Err(AppError::Unauthorized("Invalid token type".to_string()));
    }

    let user = match state.db.get_user_by_id(&claims.user_id).await {
        Ok(Some(u)) => u,
        _ => return Err(AppError::Unauthorized("User not found".to_string())),
    };

    if !user.is_active {
        return Err(AppError::Forbidden("Your account is deactivated".to_string()));
    }

    let access_token = create_access_token(&user.id, &state.settings)
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    let new_refresh_token = create_refresh_token(&user.id, &state.settings)
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(Token {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "bearer".to_string(),
    }))
}
