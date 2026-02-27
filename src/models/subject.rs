use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubjectType {
    Book,
    Report,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct Subject {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub file_path: String,
    pub is_public: bool,
    pub r#type: SubjectType,
    pub processing_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct SubjectCreate {
    pub title: String,
    pub is_public: bool,
    pub r#type: SubjectType,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct SubjectUpdate {
    pub title: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct SubjectResponse {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub file_path: String,
    pub is_public: bool,
    pub r#type: SubjectType,
    pub processing_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Subject> for SubjectResponse {
    fn from(subject: Subject) -> Self {
        SubjectResponse {
            id: subject.id,
            user_id: subject.user_id,
            title: subject.title,
            file_path: subject.file_path,
            is_public: subject.is_public,
            r#type: subject.r#type,
            processing_status: subject.processing_status,
            created_at: subject.created_at,
            updated_at: Some(subject.updated_at),
        }
    }
}
