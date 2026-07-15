use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use std::sync::Arc;

use crate::AppState;
use crate::error::AppError;
use crate::models::User;
use crate::services::auth::verify_token;

pub struct AuthUser(pub User);

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header =
            match TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await {
                Ok(TypedHeader(auth)) => auth,
                Err(_) => {
                    return Err(AppError::Unauthorized(
                        "Missing or invalid authorization header".to_string(),
                    ));
                }
            };

        let token = auth_header.token();
        let claims = verify_token(token, &state.settings)
            .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;

        if claims.r#type != "access" {
            return Err(AppError::Unauthorized("Invalid token type".to_string()));
        }

        let user = state
            .db
            .get_user_by_id(&claims.user_id)
            .await
            .map_err(AppError::InternalServerError)?;

        match user {
            Some(u) if u.is_active => Ok(AuthUser(u)),
            _ => Err(AppError::Unauthorized(
                "User not found or inactive".to_string(),
            )),
        }
    }
}

pub struct OptionalAuthUser(pub Option<User>);

impl FromRequestParts<Arc<AppState>> for OptionalAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header =
            match TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await {
                Ok(TypedHeader(auth)) => auth,
                Err(_) => return Ok(OptionalAuthUser(None)),
            };

        let token = auth_header.token();
        let claims = match verify_token(token, &state.settings) {
            Ok(c) => c,
            Err(_) => return Ok(OptionalAuthUser(None)),
        };

        if claims.r#type != "access" {
            return Ok(OptionalAuthUser(None));
        }

        let user = match state.db.get_user_by_id(&claims.user_id).await {
            Ok(Some(u)) if u.is_active => Some(u),
            _ => None,
        };

        Ok(OptionalAuthUser(user))
    }
}
