use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use std::sync::Arc;

use crate::AppState;
use crate::api::extractors::Caller;
use crate::error::AppError;
use crate::models::JobResponse;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/{job_id}", get(get_job))
}

#[utoipa::path(
    get,
    path = "/api/jobs/{job_id}",
    params(
        ("job_id" = String, Path, description = "Background job ID")
    ),
    responses(
        (status = 200, description = "Job progress", body = JobResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Jobs",
    security(
        ("bearerAuth" = []),
        ("apiKeyAuth" = [])
    )
)]
pub async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
    caller: Caller,
) -> Result<Json<JobResponse>, AppError> {
    let job = state
        .db
        .get_job(&job_id)
        .await
        .map_err(AppError::InternalServerError)?
        .ok_or_else(|| AppError::NotFound("Job not found".to_string()))?;

    if !caller.can_access_job(&job) {
        return Err(AppError::Forbidden(
            "You do not have permission to view this job".to_string(),
        ));
    }

    Ok(Json(JobResponse::from(job)))
}
