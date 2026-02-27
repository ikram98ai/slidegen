use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct Slide {
    pub chapter_id: String,
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub points: Vec<String>,
    pub explanation: String,
    pub voice_url: Option<String>,
    pub order_index: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct SlideCreate {
    pub chapter_id: String,
    pub title: String,
    pub points: Vec<String>,
    pub explanation: String,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct SlideUpdate {
    pub title: Option<String>,
    pub points: Option<Vec<String>>,
    pub explanation: Option<String>,
    pub order_index: Option<i32>,
    pub voice_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct SlideResponse {
    pub id: String,
    pub chapter_id: String,
    pub title: String,
    pub points: Vec<String>,
    pub explanation: String,
    pub voice_url: Option<String>,
    pub order_index: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Slide> for SlideResponse {
    fn from(slide: Slide) -> Self {
        SlideResponse {
            id: slide.id,
            chapter_id: slide.chapter_id,
            title: slide.title,
            points: slide.points,
            explanation: slide.explanation,
            voice_url: slide.voice_url,
            order_index: slide.order_index,
            created_at: slide.created_at,
            updated_at: Some(slide.updated_at),
        }
    }
}
