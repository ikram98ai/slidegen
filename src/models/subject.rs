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
    /// Pages extracted so far by the current processing run (None before the
    /// first run reports progress; absent on items written by older versions).
    #[serde(default)]
    pub processed_pages: Option<i32>,
    /// Total pages the current processing run will extract.
    #[serde(default)]
    pub total_pages: Option<i32>,
    /// Latest background job for this book (TOC extract, later chapter builds).
    #[serde(default)]
    pub job_id: Option<String>,
    /// Fine-grained stage mirrored from the job (`extracting_pages`, `analyzing_toc`, …).
    #[serde(default)]
    pub processing_stage: Option<String>,
    /// S3 prefix of the page-accurate extract (`…/extract`).
    #[serde(default)]
    pub extract_prefix: Option<String>,
    /// `printed_page = pdf_page - page_offset` when known.
    #[serde(default)]
    pub page_offset: Option<i32>,
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
    #[serde(default)]
    pub processed_pages: Option<i32>,
    #[serde(default)]
    pub total_pages: Option<i32>,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub processing_stage: Option<String>,
    #[serde(default)]
    pub extract_prefix: Option<String>,
    #[serde(default)]
    pub page_offset: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// A subject with its chapters embedded, as returned by
/// `GET /api/subjects/{id}/chapters`. The subject fields are flattened to the
/// top level to match the pre-migration API shape the frontend consumes.
#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct SubjectDetailResponse {
    #[serde(flatten)]
    pub subject: SubjectResponse,
    pub chapters: Vec<super::ChapterResponse>,
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
            processed_pages: subject.processed_pages,
            total_pages: subject.total_pages,
            job_id: subject.job_id,
            processing_stage: subject.processing_stage,
            extract_prefix: subject.extract_prefix,
            page_offset: subject.page_offset,
            created_at: subject.created_at,
            updated_at: Some(subject.updated_at),
        }
    }
}
