use axum::{extract::FromRequestParts, http::header::HeaderMap, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use std::sync::Arc;

use crate::AppState;
use crate::error::AppError;
use crate::models::{JobRecord, User};
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

/// Sibling backend authenticated with `X-Api-Key`.
pub struct ServiceAuth {
    pub tenant_id: String,
}

impl ServiceAuth {
    pub fn user_id(&self) -> String {
        tenant_user_id(&self.tenant_id)
    }
}

pub fn tenant_user_id(tenant_id: &str) -> String {
    format!("tenant:{tenant_id}")
}

fn api_key_from(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

impl FromRequestParts<Arc<AppState>> for ServiceAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let Some(key) = api_key_from(&parts.headers) else {
            return Err(AppError::Unauthorized(
                "Missing X-Api-Key header".to_string(),
            ));
        };

        match state.settings.tenant_for_api_key(key) {
            Some(tenant_id) => Ok(ServiceAuth {
                tenant_id: tenant_id.to_string(),
            }),
            None => Err(AppError::Unauthorized("Invalid API key".to_string())),
        }
    }
}

/// User JWT or tenant API key. Used by the jobs API so both the slidegen UI
/// and other backends can poll progress.
pub enum Caller {
    User(User),
    Service { tenant_id: String },
}

impl Caller {
    pub fn user_id(&self) -> String {
        match self {
            Self::User(u) => u.id.clone(),
            Self::Service { tenant_id } => tenant_user_id(tenant_id),
        }
    }

    pub fn tenant_id(&self) -> Option<&str> {
        match self {
            Self::User(_) => None,
            Self::Service { tenant_id } => Some(tenant_id.as_str()),
        }
    }

    pub fn can_access_job(&self, job: &JobRecord) -> bool {
        if job.user_id == self.user_id() {
            return true;
        }
        match (self.tenant_id(), job.tenant_id.as_deref()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

impl FromRequestParts<Arc<AppState>> for Caller {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        if let Some(key) = api_key_from(&parts.headers) {
            return match state.settings.tenant_for_api_key(key) {
                Some(tenant_id) => Ok(Caller::Service {
                    tenant_id: tenant_id.to_string(),
                }),
                None => Err(AppError::Unauthorized("Invalid API key".to_string())),
            };
        }

        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        Ok(Caller::User(user))
    }
}
