use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    ProcessSubject,
    GenerateSlides,
    GenerateChapter,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProcessSubject => "process_subject",
            Self::GenerateSlides => "generate_slides",
            Self::GenerateChapter => "generate_chapter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// Fine-grained stage other services poll in addition to `status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobStage {
    Queued,
    ExtractingPages,
    AnalyzingToc,
    GeneratingSlides,
    PlanningScenes,
    GeneratingScenes,
    VerifyingCitations,
    Completed,
    Failed,
}

impl JobStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::ExtractingPages => "extracting_pages",
            Self::AnalyzingToc => "analyzing_toc",
            Self::GeneratingSlides => "generating_slides",
            Self::PlanningScenes => "planning_scenes",
            Self::GeneratingScenes => "generating_scenes",
            Self::VerifyingCitations => "verifying_citations",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct JobRecord {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub stage: String,
    pub user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    pub subject_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_s3path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processed_pages: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processed_slides: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_slides: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapters_done: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapters_total: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_offset: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl JobRecord {
    pub fn new_process(
        user_id: impl Into<String>,
        tenant_id: Option<String>,
        subject_id: impl Into<String>,
        file_s3path: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: JobKind::ProcessSubject.as_str().to_string(),
            status: JobStatus::Queued.as_str().to_string(),
            stage: JobStage::Queued.as_str().to_string(),
            user_id: user_id.into(),
            tenant_id,
            subject_id: subject_id.into(),
            chapter_id: None,
            file_s3path: Some(file_s3path.into()),
            processed_pages: None,
            total_pages: None,
            processed_slides: None,
            total_slides: None,
            chapters_done: None,
            chapters_total: None,
            extract_prefix: None,
            page_offset: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_generate_slides(
        user_id: impl Into<String>,
        tenant_id: Option<String>,
        subject_id: impl Into<String>,
        chapter_id: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: JobKind::GenerateSlides.as_str().to_string(),
            status: JobStatus::Queued.as_str().to_string(),
            stage: JobStage::GeneratingSlides.as_str().to_string(),
            user_id: user_id.into(),
            tenant_id,
            subject_id: subject_id.into(),
            chapter_id: Some(chapter_id.into()),
            file_s3path: None,
            processed_pages: None,
            total_pages: None,
            processed_slides: None,
            total_slides: None,
            chapters_done: None,
            chapters_total: None,
            extract_prefix: None,
            page_offset: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_generate_chapter(
        user_id: impl Into<String>,
        tenant_id: Option<String>,
        subject_id: impl Into<String>,
        chapter_id: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: JobKind::GenerateChapter.as_str().to_string(),
            status: JobStatus::Queued.as_str().to_string(),
            stage: JobStage::GeneratingScenes.as_str().to_string(),
            user_id: user_id.into(),
            tenant_id,
            subject_id: subject_id.into(),
            chapter_id: Some(chapter_id.into()),
            file_s3path: None,
            processed_pages: None,
            total_pages: None,
            processed_slides: None,
            total_slides: None,
            chapters_done: None,
            chapters_total: None,
            extract_prefix: None,
            page_offset: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct JobResponse {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub stage: String,
    pub subject_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processed_pages: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processed_slides: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_slides: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapters_done: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapters_total: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_offset: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<JobRecord> for JobResponse {
    fn from(job: JobRecord) -> Self {
        JobResponse {
            id: job.id,
            kind: job.kind,
            status: job.status,
            stage: job.stage,
            subject_id: job.subject_id,
            chapter_id: job.chapter_id,
            tenant_id: job.tenant_id,
            processed_pages: job.processed_pages,
            total_pages: job.total_pages,
            processed_slides: job.processed_slides,
            total_slides: job.total_slides,
            chapters_done: job.chapters_done,
            chapters_total: job.chapters_total,
            extract_prefix: job.extract_prefix,
            page_offset: job.page_offset,
            error: job.error,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct IngestBookRequest {
    pub title: String,
    pub r#type: crate::models::SubjectType,
    /// Object key already in the Slidegen bucket (other services upload first).
    pub file_s3path: String,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct IngestBookResponse {
    pub book: crate::models::SubjectResponse,
    pub job: JobResponse,
}
