use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct Chapter {
    pub subject_id: String,
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub page_start: i32,
    pub page_end: i32,
    pub order_index: i32,
    pub processing_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct ChapterCreate {
    pub subject_id: String,
    pub title: String,
    pub page_start: i32,
    pub page_end: i32,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct ChapterUpdate {
    pub title: Option<String>,
    pub page_start: Option<i32>,
    pub page_end: Option<i32>,
    pub order_index: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct ChapterResponse {
    pub id: String,
    pub subject_id: String,
    pub title: String,
    pub page_start: i32,
    pub page_end: i32,
    pub order_index: i32,
    pub processing_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Chapter> for ChapterResponse {
    fn from(chapter: Chapter) -> Self {
        ChapterResponse {
            id: chapter.id,
            subject_id: chapter.subject_id,
            title: chapter.title,
            page_start: chapter.page_start,
            page_end: chapter.page_end,
            order_index: chapter.order_index,
            processing_status: chapter.processing_status,
            created_at: chapter.created_at,
            updated_at: Some(chapter.updated_at),
        }
    }
}
