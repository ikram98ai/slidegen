use crate::config::Settings;
use crate::db::Database;
use crate::models::{
    Chapter, ChapterManifest, JobRecord, JobStage, JobStatus, SceneSpec, Slide, Subject,
    SubjectType, chapter_audio_key, chapter_manifest_key, chapter_package_html_key,
};
use crate::services::citations::{
    ParagraphIndex, ground_scenes, ground_scenes_against_text, prompt_from_pages, prompt_from_plan,
};
use crate::services::compiler::compile_chapter;
use crate::services::events::{BookProcessed, ChapterReady, EventBus, JobFailed};
use crate::services::extract::{
    self, ExtractManifest, ExtractedPage, arabic_from_label, extract_prefix, manifest_object_key,
    page_object_key,
};
use crate::services::retrieval::RetrievalService;
use crate::services::{AIService, StorageService};
use anyhow::{Context, Result};
use chrono::Utc;
use dotext::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

fn spawn_event_worker(bus: EventBus) -> mpsc::UnboundedSender<(String, serde_json::Value)> {
    let (tx, mut rx) = mpsc::unbounded_channel::<(String, serde_json::Value)>();
    tokio::spawn(async move {
        while let Some((detail_type, detail)) = rx.recv().await {
            if let Err(e) = bus.publish(&detail_type, &detail).await {
                tracing::warn!(
                    error = format!("{e:#}"),
                    detail_type,
                    "Failed to publish EventBridge event"
                );
            }
        }
    });
    tx
}

/// Truncates a string to at most `max_bytes` without splitting a UTF-8
/// character (a plain `&s[..max_bytes]` panics on non-ASCII boundaries).
fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// A background job, either run in-process (local dev) or shipped to SQS and
/// executed by the worker Lambda.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Job {
    ProcessSubject {
        user_id: String,
        subject_id: String,
        file_s3path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        job_id: Option<String>,
    },
    GenerateSlides {
        user_id: String,
        subject_id: String,
        chapter_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        job_id: Option<String>,
    },
    GenerateChapter {
        user_id: String,
        subject_id: String,
        chapter_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        job_id: Option<String>,
    },
    /// Sibling services can drop this on the jobs queue instead of calling HTTP.
    IngestBook {
        tenant_id: String,
        title: String,
        #[serde(rename = "book_type")]
        book_type: SubjectType,
        file_s3path: String,
        #[serde(default)]
        is_public: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        job_id: Option<String>,
    },
}

/// SQS-backed job queue.
#[derive(Clone, Debug)]
pub struct JobQueue {
    client: aws_sdk_sqs::Client,
    queue_url: String,
}

impl JobQueue {
    pub async fn new(settings: &Settings, queue_url: String) -> Self {
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(settings.aws_region.clone()))
            .load()
            .await;

        JobQueue {
            client: aws_sdk_sqs::Client::new(&sdk_config),
            queue_url,
        }
    }

    pub async fn send(&self, job: &Job) -> Result<()> {
        let body = serde_json::to_string(job).context("Failed to serialize job")?;
        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(body)
            .send()
            .await
            .context("Failed to enqueue job to SQS")?;
        Ok(())
    }

    pub async fn receive(
        &self,
        max_messages: i32,
        wait_seconds: i32,
        visibility_timeout: i32,
    ) -> Result<Vec<QueueMessage>> {
        let resp = self
            .client
            .receive_message()
            .queue_url(&self.queue_url)
            .max_number_of_messages(max_messages)
            .wait_time_seconds(wait_seconds)
            .visibility_timeout(visibility_timeout)
            .send()
            .await
            .context("Failed to receive SQS messages")?;
        Ok(resp
            .messages
            .unwrap_or_default()
            .into_iter()
            .filter_map(|m| {
                Some(QueueMessage {
                    message_id: m.message_id.unwrap_or_default(),
                    receipt_handle: m.receipt_handle?,
                    body: m.body.unwrap_or_default(),
                })
            })
            .collect())
    }

    pub async fn delete(&self, receipt_handle: &str) -> Result<()> {
        self.client
            .delete_message()
            .queue_url(&self.queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await
            .context("Failed to delete SQS message")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct QueueMessage {
    pub message_id: String,
    pub receipt_handle: String,
    pub body: String,
}

#[derive(Clone)]
pub struct BackgroundTasksService {
    db: Arc<Database>,
    storage: Arc<StorageService>,
    ai: Arc<AIService>,
    queue: Option<JobQueue>,
    local_tx: Option<mpsc::UnboundedSender<Job>>,
    event_tx: Option<mpsc::UnboundedSender<(String, serde_json::Value)>>,
    auto_generate_chapters: bool,
    retrieval: Option<Arc<RetrievalService>>,
}

impl BackgroundTasksService {
    pub fn new(
        db: Arc<Database>,
        storage: Arc<StorageService>,
        ai: Arc<AIService>,
        queue: Option<JobQueue>,
    ) -> Self {
        let retrieval = Arc::new(RetrievalService::new(
            db.clone(),
            storage.clone(),
            ai.clone(),
            None,
        ));
        Self::with_events(db, storage, ai, queue, None, true, Some(retrieval))
    }

    pub fn with_events(
        db: Arc<Database>,
        storage: Arc<StorageService>,
        ai: Arc<AIService>,
        queue: Option<JobQueue>,
        events: Option<EventBus>,
        auto_generate_chapters: bool,
        retrieval: Option<Arc<RetrievalService>>,
    ) -> Self {
        let (local_tx, local_rx) = if queue.is_none() {
            let (tx, rx) = mpsc::unbounded_channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };
        let service = Self {
            db,
            storage,
            ai,
            queue,
            local_tx,
            event_tx: events.map(spawn_event_worker),
            auto_generate_chapters,
            retrieval,
        };
        if let Some(mut rx) = local_rx {
            let worker = service.clone();
            tokio::spawn(async move {
                while let Some(job) = rx.recv().await {
                    let _ = worker.run_job(job).await;
                }
            });
        }
        service
    }

    /// Creates a tracked job row and hands the work to SQS / in-process.
    pub async fn start_process_subject(
        &self,
        user_id: String,
        tenant_id: Option<String>,
        subject_id: String,
        file_s3path: String,
    ) -> Result<JobRecord> {
        let job = JobRecord::new_process(
            user_id.clone(),
            tenant_id,
            subject_id.clone(),
            file_s3path.clone(),
        );
        self.db.save_job(&job).await?;
        self.sync_subject_job(&subject_id, &job).await;
        self.dispatch(Job::ProcessSubject {
            user_id,
            subject_id,
            file_s3path,
            job_id: Some(job.id.clone()),
        })
        .await?;
        Ok(job)
    }

    pub async fn start_generate_slides(
        &self,
        user_id: String,
        tenant_id: Option<String>,
        subject_id: String,
        chapter_id: String,
    ) -> Result<JobRecord> {
        let job = JobRecord::new_generate_slides(
            user_id.clone(),
            tenant_id,
            subject_id.clone(),
            chapter_id.clone(),
        );
        self.db.save_job(&job).await?;
        self.dispatch(Job::GenerateSlides {
            user_id,
            subject_id,
            chapter_id,
            job_id: Some(job.id.clone()),
        })
        .await?;
        Ok(job)
    }

    pub async fn start_generate_chapter(
        &self,
        user_id: String,
        tenant_id: Option<String>,
        subject_id: String,
        chapter_id: String,
    ) -> Result<JobRecord> {
        let job = JobRecord::new_generate_chapter(
            user_id.clone(),
            tenant_id,
            subject_id.clone(),
            chapter_id.clone(),
        );
        self.db.save_job(&job).await?;
        self.dispatch(Job::GenerateChapter {
            user_id,
            subject_id,
            chapter_id,
            job_id: Some(job.id.clone()),
        })
        .await?;
        Ok(job)
    }

    /// Hands a job off for background execution: to SQS when a queue is
    /// configured (required on Lambda, where in-process tasks stall once the
    /// response is sent), otherwise to the in-process job channel.
    pub async fn dispatch(&self, job: Job) -> Result<()> {
        if let Some(queue) = &self.queue {
            tracing::info!(?job, "Dispatching job to SQS");
            return queue.send(&job).await;
        }
        if let Some(tx) = &self.local_tx {
            tracing::info!(?job, "Running job in-process");
            tx.send(job)
                .map_err(|_| anyhow::anyhow!("in-process job worker dropped"))?;
            return Ok(());
        }
        anyhow::bail!("no job queue or in-process worker configured")
    }

    /// Executes a job to completion. On failure the related entity is marked
    /// "failed" and the error is returned so SQS consumers can retry.
    pub async fn run_job(&self, job: Job) -> Result<()> {
        match job {
            Job::ProcessSubject {
                user_id,
                subject_id,
                file_s3path,
                job_id,
            } => {
                self.process_subject_bg(user_id, subject_id, file_s3path, job_id)
                    .await
            }
            Job::GenerateSlides {
                user_id,
                subject_id,
                chapter_id,
                job_id,
            } => {
                let chapter = self
                    .db
                    .get_chapter(&subject_id, &chapter_id)
                    .await?
                    .context("Chapter not found")?;
                self.generate_slides_bg(user_id, subject_id, chapter, job_id)
                    .await
            }
            Job::GenerateChapter {
                user_id,
                subject_id,
                chapter_id,
                job_id,
            } => {
                let chapter = self
                    .db
                    .get_chapter(&subject_id, &chapter_id)
                    .await?
                    .context("Chapter not found")?;
                self.generate_chapter_bg(user_id, subject_id, chapter, job_id)
                    .await
            }
            Job::IngestBook {
                tenant_id,
                title,
                book_type,
                file_s3path,
                is_public,
                job_id,
            } => {
                self.ingest_book_bg(tenant_id, title, book_type, file_s3path, is_public, job_id)
                    .await
            }
        }
    }

    pub async fn process_subject_bg(
        &self,
        user_id: String,
        subject_id: String,
        file_s3path: String,
        job_id: Option<String>,
    ) -> Result<()> {
        tracing::info!(
            user_id = %user_id,
            subject_id = %subject_id,
            file_s3path = %file_s3path,
            "Background: processing subject file"
        );

        if let Err(e) = self
            .do_process_subject(&user_id, &subject_id, &file_s3path, job_id.as_deref())
            .await
        {
            tracing::error!(
                error = format!("{e:#}"),
                "Failed to process subject background task"
            );
            self.fail_job(job_id.as_deref(), &e).await;
            self.emit(
                "job.failed",
                &JobFailed {
                    job_id: job_id.clone(),
                    kind: "process_subject".into(),
                    error: format!("{e:#}"),
                    book_id: Some(subject_id.clone()),
                    chapter_id: None,
                },
            )
            .await;
            if let Ok(Some(mut subject)) = self.db.get_subject(&subject_id).await {
                subject.processing_status = "failed".to_string();
                subject.processing_stage = Some(JobStage::Failed.as_str().to_string());
                let _ = self.db.save_subject(&subject).await;
            }
            return Err(e);
        }
        Ok(())
    }

    async fn extract_text(&self, file_data: &[u8], extension: &str) -> Result<String> {
        match extension.to_lowercase().as_str() {
            "pdf" => {
                let doc =
                    lopdf::Document::load_mem(file_data).context("Failed to load PDF document")?;
                let total = doc.get_pages().len() as u32;
                let page_numbers: Vec<u32> = (1..=total).collect();
                doc.extract_text(&page_numbers)
                    .context("Failed to extract PDF text")
            }
            "docx" => {
                let mut tmpfile = tempfile::NamedTempFile::new()
                    .context("Failed to create temporary file for DOCX")?;
                tmpfile
                    .write_all(file_data)
                    .context("Failed to write DOCX data to temp file")?;

                let mut reader = Docx::open(tmpfile.path())
                    .map_err(|e| anyhow::anyhow!("Failed to open DOCX: {:?}", e))?;

                let mut text = String::new();
                reader
                    .read_to_string(&mut text)
                    .context("Failed to read text from DOCX")?;
                Ok(text)
            }
            _ => anyhow::bail!("Unsupported file extension: {}", extension),
        }
    }

    /// Extracts every PDF page, writes paragraph-level JSON to S3, and returns
    /// the in-memory pages plus manifest used to build the TOC prompt.
    async fn extract_pdf_book(
        &self,
        file_data: &[u8],
        user_id: &str,
        subject_id: &str,
        job_id: Option<&str>,
    ) -> Result<(Vec<ExtractedPage>, ExtractManifest)> {
        let doc = lopdf::Document::load_mem(file_data).context("Failed to load PDF document")?;
        let total = doc.get_pages().len() as u32;
        if total == 0 {
            anyhow::bail!("PDF has no pages");
        }

        self.update_page_progress(
            subject_id,
            job_id,
            0,
            total as i32,
            JobStage::ExtractingPages,
        )
        .await;

        let mut pages = Vec::with_capacity(total as usize);
        for page in 1..=total {
            match doc.extract_text(&[page]) {
                Ok(page_text) => pages.push(extract::page_from_text(page, &page_text)),
                Err(e) => {
                    tracing::warn!(
                        error = format!("{e:#}"),
                        page,
                        "Skipping unextractable page"
                    );
                    pages.push(ExtractedPage {
                        pdf_page: page,
                        printed_page: None,
                        printed_label: None,
                        paragraphs: vec![],
                    });
                }
            }
            self.update_page_progress(
                subject_id,
                job_id,
                page as i32,
                total as i32,
                JobStage::ExtractingPages,
            )
            .await;
        }

        if pages.iter().all(|p| p.paragraphs.is_empty()) {
            anyhow::bail!("Failed to extract text from any of the {total} PDF pages");
        }

        let offset_samples: Vec<(u32, Option<i32>)> = pages
            .iter()
            .map(|p| (p.pdf_page, arabic_from_label(p.printed_label.as_deref())))
            .collect();
        let page_offset = extract::infer_page_offset(&offset_samples);
        extract::apply_page_offset(&mut pages, page_offset);
        let toc_pdf_pages = extract::detect_toc_pages(&pages);
        let prefix = extract_prefix(user_id, subject_id);
        let manifest = ExtractManifest {
            total_pages: total,
            page_offset,
            toc_pdf_pages,
            pages_prefix: prefix.clone(),
        };

        for page in &pages {
            let key = page_object_key(&prefix, page.pdf_page);
            self.storage
                .upload_json(&key, page)
                .await
                .with_context(|| format!("Failed to upload extract page {}", page.pdf_page))?;
        }
        self.storage
            .upload_json(&manifest_object_key(&prefix), &manifest)
            .await
            .context("Failed to upload extract manifest")?;

        Ok((pages, manifest))
    }

    async fn update_page_progress(
        &self,
        subject_id: &str,
        job_id: Option<&str>,
        processed: i32,
        total: i32,
        stage: JobStage,
    ) {
        if let Ok(Some(mut subject)) = self.db.get_subject(subject_id).await {
            subject.processed_pages = Some(processed);
            subject.total_pages = Some(total);
            subject.processing_status = "processing".to_string();
            subject.processing_stage = Some(stage.as_str().to_string());
            subject.updated_at = Utc::now();
            let _ = self.db.save_subject(&subject).await;
        }
        if let Some(id) = job_id
            && let Ok(Some(mut job)) = self.db.get_job(id).await
        {
            job.status = JobStatus::Running.as_str().to_string();
            job.stage = stage.as_str().to_string();
            job.processed_pages = Some(processed);
            job.total_pages = Some(total);
            job.updated_at = Utc::now();
            let _ = self.db.save_job(&job).await;
        }
    }

    /// Best-effort slide/scene progress write for a chapter's generation run.
    async fn update_slide_progress(
        &self,
        subject_id: &str,
        chapter_id: &str,
        job_id: Option<&str>,
        processed: i32,
        total: i32,
    ) {
        self.update_generation_progress(
            subject_id,
            chapter_id,
            job_id,
            processed,
            total,
            JobStage::GeneratingSlides,
        )
        .await;
    }

    async fn update_scene_progress(
        &self,
        subject_id: &str,
        chapter_id: &str,
        job_id: Option<&str>,
        processed: i32,
        total: i32,
    ) {
        self.update_generation_progress(
            subject_id,
            chapter_id,
            job_id,
            processed,
            total,
            JobStage::GeneratingScenes,
        )
        .await;
    }

    async fn update_generation_progress(
        &self,
        subject_id: &str,
        chapter_id: &str,
        job_id: Option<&str>,
        processed: i32,
        total: i32,
        stage: JobStage,
    ) {
        if let Ok(Some(mut chapter)) = self.db.get_chapter(subject_id, chapter_id).await {
            chapter.processed_slides = Some(processed);
            chapter.total_slides = Some(total);
            chapter.updated_at = Utc::now();
            let _ = self.db.save_chapter(&chapter).await;
        }
        if let Some(id) = job_id
            && let Ok(Some(mut job)) = self.db.get_job(id).await
        {
            job.status = JobStatus::Running.as_str().to_string();
            job.stage = stage.as_str().to_string();
            job.processed_slides = Some(processed);
            job.total_slides = Some(total);
            job.updated_at = Utc::now();
            let _ = self.db.save_job(&job).await;
        }
    }

    async fn fail_job(&self, job_id: Option<&str>, error: &anyhow::Error) {
        let Some(id) = job_id else { return };
        if let Ok(Some(mut job)) = self.db.get_job(id).await {
            job.status = JobStatus::Failed.as_str().to_string();
            job.stage = JobStage::Failed.as_str().to_string();
            job.error = Some(format!("{error:#}"));
            job.updated_at = Utc::now();
            let _ = self.db.save_job(&job).await;
        }
    }

    async fn complete_job(&self, job_id: Option<&str>, chapters_total: Option<i32>) {
        let Some(id) = job_id else { return };
        if let Ok(Some(mut job)) = self.db.get_job(id).await {
            job.status = JobStatus::Completed.as_str().to_string();
            job.stage = JobStage::Completed.as_str().to_string();
            job.chapters_total = chapters_total.or(job.chapters_total);
            job.chapters_done = chapters_total.or(job.chapters_done);
            job.error = None;
            job.updated_at = Utc::now();
            let _ = self.db.save_job(&job).await;
        }
    }

    async fn sync_subject_job(&self, subject_id: &str, job: &JobRecord) {
        if let Ok(Some(mut subject)) = self.db.get_subject(subject_id).await {
            subject.job_id = Some(job.id.clone());
            subject.processing_stage = Some(job.stage.clone());
            subject.updated_at = Utc::now();
            let _ = self.db.save_subject(&subject).await;
        }
    }

    async fn mark_job_stage(&self, job_id: Option<&str>, stage: JobStage) {
        if let Some(id) = job_id
            && let Ok(Some(mut job)) = self.db.get_job(id).await
        {
            job.status = JobStatus::Running.as_str().to_string();
            job.stage = stage.as_str().to_string();
            job.updated_at = Utc::now();
            let _ = self.db.save_job(&job).await;
        }
    }

    async fn do_process_subject(
        &self,
        user_id: &str,
        subject_id: &str,
        file_s3path: &str,
        job_id: Option<&str>,
    ) -> Result<()> {
        let file_data = self
            .storage
            .download_file(file_s3path)
            .await
            .context("Failed to download file from S3")?;

        let extension = file_s3path.split('.').next_back().unwrap_or("");
        let toc_text = if extension.eq_ignore_ascii_case("pdf") {
            let (pages, manifest) = self
                .extract_pdf_book(&file_data, user_id, subject_id, job_id)
                .await?;

            if let Ok(Some(mut subject)) = self.db.get_subject(subject_id).await {
                subject.extract_prefix = Some(manifest.pages_prefix.clone());
                subject.page_offset = manifest.page_offset;
                subject.total_pages = Some(manifest.total_pages as i32);
                subject.processed_pages = Some(manifest.total_pages as i32);
                subject.processing_stage = Some(JobStage::AnalyzingToc.as_str().to_string());
                subject.updated_at = Utc::now();
                let _ = self.db.save_subject(&subject).await;
            }
            if let Some(id) = job_id
                && let Ok(Some(mut job)) = self.db.get_job(id).await
            {
                job.extract_prefix = Some(manifest.pages_prefix.clone());
                job.page_offset = manifest.page_offset;
                job.total_pages = Some(manifest.total_pages as i32);
                job.processed_pages = Some(manifest.total_pages as i32);
                job.stage = JobStage::AnalyzingToc.as_str().to_string();
                job.status = JobStatus::Running.as_str().to_string();
                job.updated_at = Utc::now();
                let _ = self.db.save_job(&job).await;
            }

            extract::build_toc_prompt(&pages, &manifest)
        } else {
            self.mark_job_stage(job_id, JobStage::AnalyzingToc).await;
            truncate_to_char_boundary(&self.extract_text(&file_data, extension).await?, 48_000)
                .to_string()
        };

        let chapters_data = self
            .ai
            .analyze_book_toc(&toc_text)
            .await
            .context("AI failed to analyze TOC")?;

        let mut chapters = Vec::new();
        for (idx, c_data) in chapters_data.into_iter().enumerate() {
            chapters.push(Chapter {
                id: Uuid::new_v4().to_string(),
                subject_id: subject_id.to_string(),
                user_id: user_id.to_string(),
                title: c_data.title,
                page_start: c_data.page_start,
                page_end: c_data.page_end,
                order_index: idx as i32,
                processing_status: Some("completed".to_string()),
                processed_slides: None,
                total_slides: None,
                job_id: None,
                package_key: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }
        let chapters_total = chapters.len() as i32;
        self.db
            .save_chapters(chapters)
            .await
            .context("Failed to save chapters to DB")?;

        if let Some(mut subject) = self.db.get_subject(subject_id).await? {
            subject.processing_status = "completed".to_string();
            subject.processing_stage = Some(JobStage::Completed.as_str().to_string());
            subject.updated_at = Utc::now();
            self.db
                .save_subject(&subject)
                .await
                .context("Failed to update subject status")?;
        }

        self.complete_job(job_id, Some(chapters_total)).await;
        let tenant_id = self.tenant_for_job(user_id, job_id).await;
        self.emit(
            "book.processed",
            &BookProcessed {
                book_id: subject_id.to_string(),
                user_id: user_id.to_string(),
                tenant_id: tenant_id.clone(),
                job_id: job_id.map(str::to_string),
                chapters_total,
            },
        )
        .await;
        self.maybe_enqueue_next(user_id, tenant_id, subject_id, "first chapter after TOC")
            .await;
        Ok(())
    }

    async fn ingest_book_bg(
        &self,
        tenant_id: String,
        title: String,
        subject_type: SubjectType,
        file_s3path: String,
        is_public: bool,
        job_id: Option<String>,
    ) -> Result<()> {
        let user_id = format!("tenant:{tenant_id}");
        let subject_id = Uuid::new_v4().to_string();
        let subject = Subject {
            id: subject_id.clone(),
            user_id: user_id.clone(),
            title,
            file_path: file_s3path.clone(),
            is_public,
            r#type: subject_type,
            processing_status: "processing".to_string(),
            processed_pages: None,
            total_pages: None,
            job_id: job_id.clone(),
            processing_stage: Some(JobStage::Queued.as_str().to_string()),
            extract_prefix: None,
            page_offset: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.db
            .save_subject(&subject)
            .await
            .context("Failed to save ingested subject")?;
        self.process_subject_bg(user_id, subject_id, file_s3path, job_id)
            .await
    }

    async fn maybe_enqueue_next(
        &self,
        user_id: &str,
        tenant_id: Option<String>,
        subject_id: &str,
        label: &str,
    ) {
        if !self.auto_generate_chapters {
            return;
        }
        if let Err(e) = self
            .enqueue_next_chapter(user_id, tenant_id, subject_id)
            .await
        {
            tracing::warn!(
                error = format!("{e:#}"),
                subject_id,
                reason = label,
                "Failed to enqueue sequential chapter"
            );
        }
    }

    async fn enqueue_next_chapter(
        &self,
        user_id: &str,
        tenant_id: Option<String>,
        subject_id: &str,
    ) -> Result<()> {
        let mut chapters = self.db.get_chapters_by_subject(subject_id).await?;
        chapters.sort_by_key(|c| c.order_index);
        let Some(next) = chapters.into_iter().find(|c| {
            c.package_key.is_none()
                && matches!(c.processing_status.as_deref(), None | Some("completed"))
        }) else {
            return Ok(());
        };

        if let Ok(Some(mut chapter)) = self.db.get_chapter(subject_id, &next.id).await {
            chapter.processing_status = Some("processing".to_string());
            chapter.processed_slides = None;
            chapter.total_slides = None;
            chapter.updated_at = Utc::now();
            let _ = self.db.save_chapter(&chapter).await;
        }

        self.start_generate_chapter(
            user_id.to_string(),
            tenant_id,
            subject_id.to_string(),
            next.id,
        )
        .await?;
        Ok(())
    }

    async fn tenant_for_job(&self, user_id: &str, job_id: Option<&str>) -> Option<String> {
        if let Some(id) = job_id
            && let Ok(Some(job)) = self.db.get_job(id).await
            && let Some(tenant) = job.tenant_id
        {
            return Some(tenant);
        }
        user_id.strip_prefix("tenant:").map(str::to_string)
    }

    async fn emit<T: serde::Serialize>(&self, detail_type: &str, detail: &T) {
        let Some(tx) = &self.event_tx else {
            return;
        };
        match serde_json::to_value(detail) {
            Ok(value) => {
                if tx.send((detail_type.to_string(), value)).is_err() {
                    tracing::warn!(detail_type, "Event worker dropped; event not published");
                }
            }
            Err(e) => tracing::warn!(error = %e, detail_type, "Failed to serialize event"),
        }
    }

    pub async fn generate_slides_bg(
        &self,
        user_id: String,
        subject_id: String,
        chapter: Chapter,
        job_id: Option<String>,
    ) -> Result<()> {
        tracing::info!(
            user_id = %user_id,
            subject_id = %subject_id,
            chapter_id = %chapter.id,
            "Background: generating slides for chapter"
        );

        if let Err(e) = self
            .do_generate_slides(&user_id, &subject_id, &chapter, job_id.as_deref())
            .await
        {
            tracing::error!(
                error = format!("{e:#}"),
                "Failed to generate slides background task"
            );
            self.fail_job(job_id.as_deref(), &e).await;
            if let Ok(Some(mut c)) = self.db.get_chapter(&subject_id, &chapter.id).await {
                c.processing_status = Some("failed".to_string());
                let _ = self.db.save_chapter(&c).await;
            }
            return Err(e);
        }
        Ok(())
    }

    async fn do_generate_slides(
        &self,
        user_id: &str,
        subject_id: &str,
        chapter: &Chapter,
        job_id: Option<&str>,
    ) -> Result<()> {
        let subject = self
            .db
            .get_subject(subject_id)
            .await?
            .context("Subject not found")?;

        if let Ok(Some(mut c)) = self.db.get_chapter(subject_id, &chapter.id).await {
            c.job_id = job_id.map(|s| s.to_string());
            let _ = self.db.save_chapter(&c).await;
        }

        let chapter_text = self.chapter_text_for_generation(&subject, chapter).await?;

        if chapter_text.trim().is_empty() {
            anyhow::bail!("No text extracted for chapter in {}", subject.file_path);
        }

        // 4. Call AI to generate slides
        let slides_data = self
            .ai
            .generate_slides(&chapter_text)
            .await
            .context("AI failed to generate slides")?;

        let total_slides = slides_data.len() as i32;
        self.update_slide_progress(subject_id, &chapter.id, job_id, 0, total_slides)
            .await;

        // 5. Replace any slides from a previous generation run, so regenerating
        // does not accumulate duplicates.
        let old_slides = self
            .db
            .get_slides_by_chapter(&chapter.id)
            .await
            .unwrap_or_default();
        for old in old_slides {
            if let Some(voice_url) = &old.voice_url {
                let _ = self.storage.delete_file(voice_url).await;
            }
            let _ = self.db.delete_slide(&chapter.id, &old.id).await;
        }

        // 6. Save slides and generate audio
        for (idx, s_data) in slides_data.into_iter().enumerate() {
            let slide_id = Uuid::new_v4().to_string();
            let mut voice_url = None;

            // Generate Audio (returned as ready-to-play WAV bytes)
            let audio_text = format!("{}. {}", s_data.title, s_data.explanation);
            match self.ai.generate_slide_audio(&audio_text).await {
                Ok(wav_bytes) => {
                    let audio_key = format!("audio/{}/{}/{}.wav", user_id, chapter.id, slide_id);
                    if self
                        .storage
                        .upload_file(&audio_key, wav_bytes)
                        .await
                        .is_ok()
                    {
                        voice_url = Some(audio_key);
                    }
                }
                Err(e) => {
                    tracing::warn!(error = format!("{e:#}"), slide_id = %slide_id, "Failed to generate slide audio");
                }
            }

            let slide = Slide {
                id: slide_id,
                chapter_id: chapter.id.clone(),
                user_id: user_id.to_string(),
                title: s_data.title,
                points: s_data.bullets,
                explanation: s_data.explanation,
                voice_url,
                order_index: idx as i32,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            self.db
                .save_slide(&slide)
                .await
                .context("Failed to save slide")?;

            self.update_slide_progress(
                subject_id,
                &chapter.id,
                job_id,
                idx as i32 + 1,
                total_slides,
            )
            .await;
        }

        if let Some(mut c) = self.db.get_chapter(subject_id, &chapter.id).await? {
            c.processing_status = Some("completed".to_string());
            c.updated_at = Utc::now();
            self.db
                .save_chapter(&c)
                .await
                .context("Failed to update chapter status")?;
        }

        self.complete_job(job_id, None).await;
        Ok(())
    }

    pub async fn generate_chapter_bg(
        &self,
        user_id: String,
        subject_id: String,
        chapter: Chapter,
        job_id: Option<String>,
    ) -> Result<()> {
        tracing::info!(
            user_id = %user_id,
            subject_id = %subject_id,
            chapter_id = %chapter.id,
            "Background: generating interactive chapter"
        );

        if let Err(e) = self
            .do_generate_chapter(&user_id, &subject_id, &chapter, job_id.as_deref())
            .await
        {
            tracing::error!(
                error = format!("{e:#}"),
                "Failed to generate chapter background task"
            );
            self.fail_job(job_id.as_deref(), &e).await;
            self.emit(
                "job.failed",
                &JobFailed {
                    job_id: job_id.clone(),
                    kind: "generate_chapter".into(),
                    error: format!("{e:#}"),
                    book_id: Some(subject_id.clone()),
                    chapter_id: Some(chapter.id.clone()),
                },
            )
            .await;
            if let Ok(Some(mut c)) = self.db.get_chapter(&subject_id, &chapter.id).await {
                c.processing_status = Some("failed".to_string());
                let _ = self.db.save_chapter(&c).await;
            }
            return Err(e);
        }
        Ok(())
    }

    async fn do_generate_chapter(
        &self,
        user_id: &str,
        subject_id: &str,
        chapter: &Chapter,
        job_id: Option<&str>,
    ) -> Result<()> {
        let subject = self
            .db
            .get_subject(subject_id)
            .await?
            .context("Subject not found")?;

        if let Ok(Some(mut c)) = self.db.get_chapter(subject_id, &chapter.id).await {
            c.job_id = job_id.map(str::to_string);
            let _ = self.db.save_chapter(&c).await;
        }

        let (chapter_text, pages) = self
            .chapter_source_for_generation(&subject, chapter)
            .await?;
        if chapter_text.trim().is_empty() {
            anyhow::bail!("No text extracted for chapter in {}", subject.file_path);
        }

        let index = if pages.is_empty() {
            None
        } else {
            Some(ParagraphIndex::from_pages(&pages))
        };

        self.mark_job_stage(job_id, JobStage::PlanningScenes).await;
        let generate_text = match &index {
            Some(index) if !index.is_empty() => {
                match self
                    .ai
                    .plan_chapter_scenes(&chapter.title, &index.catalog())
                    .await
                {
                    Ok(plan) => {
                        let retrieved = prompt_from_plan(&plan, index);
                        if retrieved.len() < 200 {
                            tracing::warn!(
                                chapter_id = %chapter.id,
                                "Planner retrieved too little text; using full chapter extract"
                            );
                            chapter_text.clone()
                        } else {
                            tracing::info!(
                                chapter_id = %chapter.id,
                                scenes = plan.scenes.len(),
                                "Generating chapter from retrieved paragraphs"
                            );
                            retrieved
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = format!("{e:#}"),
                            chapter_id = %chapter.id,
                            "Scene planner failed; using full chapter extract"
                        );
                        chapter_text.clone()
                    }
                }
            }
            _ => chapter_text.clone(),
        };

        self.mark_job_stage(job_id, JobStage::GeneratingScenes)
            .await;
        let generated = self
            .ai
            .generate_chapter(&chapter.title, &generate_text)
            .await
            .context("AI failed to generate chapter scenes")?;

        if generated.scenes.is_empty() {
            anyhow::bail!("Model returned no scenes for chapter {}", chapter.id);
        }

        let mut scenes: Vec<SceneSpec> = generated
            .scenes
            .into_iter()
            .enumerate()
            .map(|(idx, scene)| SceneSpec::from_generated(scene, idx))
            .collect();

        self.mark_job_stage(job_id, JobStage::VerifyingCitations)
            .await;
        if let Some(index) = &index {
            let stats = ground_scenes(&mut scenes, index);
            tracing::info!(
                chapter_id = %chapter.id,
                verified = stats.verified,
                repaired = stats.repaired,
                dropped_scenes = stats.dropped_scenes,
                "Grounded chapter citations against extract"
            );
        } else {
            ground_scenes_against_text(&mut scenes, &chapter_text);
        }

        if scenes.is_empty() {
            anyhow::bail!(
                "No grounded scenes remain after citation verification for chapter {}",
                chapter.id
            );
        }

        let mut manifest = ChapterManifest::new(
            subject.id.clone(),
            subject.title.clone(),
            chapter.id.clone(),
            chapter.title.clone(),
            generated.kid_lede,
            chapter.order_index,
            chapter.page_start,
            chapter.page_end,
            scenes,
        );

        let total_scenes = manifest.scenes.len() as i32;
        self.update_scene_progress(subject_id, &chapter.id, job_id, 0, total_scenes)
            .await;

        for (idx, scene) in manifest.scenes.iter_mut().enumerate() {
            let audio_text = format!("{}. {}", scene.title, scene.depth.text);
            match self.ai.generate_slide_audio(&audio_text).await {
                Ok(wav_bytes) => {
                    let audio_key = chapter_audio_key(user_id, subject_id, &chapter.id, &scene.id);
                    if self
                        .storage
                        .upload_file(&audio_key, wav_bytes)
                        .await
                        .is_ok()
                    {
                        scene.depth.audio_key = Some(audio_key);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = format!("{e:#}"),
                        scene_id = %scene.id,
                        "Failed to generate scene audio"
                    );
                }
            }
            self.update_scene_progress(
                subject_id,
                &chapter.id,
                job_id,
                idx as i32 + 1,
                total_scenes,
            )
            .await;
        }

        let html_key = self
            .compile_and_upload_chapter(
                user_id,
                subject_id,
                &chapter.id,
                &mut manifest,
                7 * 24 * 3600,
            )
            .await?;

        if let Some(mut c) = self.db.get_chapter(subject_id, &chapter.id).await? {
            c.processing_status = Some("completed".to_string());
            c.package_key = Some(html_key.clone());
            c.updated_at = Utc::now();
            self.db
                .save_chapter(&c)
                .await
                .context("Failed to update chapter status")?;
        }

        self.complete_job(job_id, None).await;
        let tenant_id = self.tenant_for_job(user_id, job_id).await;
        if let Some(retrieval) = &self.retrieval
            && let Err(e) = retrieval
                .index_chapter(
                    tenant_id.as_deref(),
                    &subject,
                    chapter,
                    &pages,
                    &manifest.scenes,
                )
                .await
        {
            tracing::warn!(
                error = format!("{e:#}"),
                chapter_id = %chapter.id,
                "Failed to index chapter embeddings"
            );
        }
        self.emit(
            "chapter.ready",
            &ChapterReady {
                book_id: subject_id.to_string(),
                chapter_id: chapter.id.clone(),
                user_id: user_id.to_string(),
                tenant_id: tenant_id.clone(),
                job_id: job_id.map(str::to_string),
                package_key: html_key,
            },
        )
        .await;
        self.maybe_enqueue_next(user_id, tenant_id, subject_id, "next chapter")
            .await;
        Ok(())
    }

    /// Signs audio URLs, compiles HTML, and writes both the durable manifest
    /// (without ephemeral URLs) and the chapter package.
    pub async fn compile_and_upload_chapter(
        &self,
        user_id: &str,
        subject_id: &str,
        chapter_id: &str,
        manifest: &mut ChapterManifest,
        audio_ttl_secs: u64,
    ) -> Result<String> {
        self.sign_scene_audio(manifest, audio_ttl_secs).await;

        let html = compile_chapter(manifest).context("Failed to compile chapter HTML")?;
        let html_key = chapter_package_html_key(user_id, subject_id, chapter_id);
        let manifest_key = chapter_manifest_key(user_id, subject_id, chapter_id);

        let mut stored = manifest.clone();
        stored.strip_ephemeral_urls();
        self.storage
            .upload_json(&manifest_key, &stored)
            .await
            .context("Failed to upload chapter manifest")?;
        self.storage
            .upload_file(&html_key, html.into_bytes())
            .await
            .context("Failed to upload chapter HTML")?;
        Ok(html_key)
    }

    pub async fn refresh_chapter_package(
        &self,
        user_id: &str,
        subject_id: &str,
        chapter_id: &str,
        audio_ttl_secs: u64,
    ) -> Result<String> {
        let manifest_key = chapter_manifest_key(user_id, subject_id, chapter_id);
        let bytes = self
            .storage
            .download_file(&manifest_key)
            .await
            .context("Chapter manifest not found")?;
        let mut manifest: ChapterManifest =
            serde_json::from_slice(&bytes).context("Failed to parse chapter manifest")?;
        self.compile_and_upload_chapter(
            user_id,
            subject_id,
            chapter_id,
            &mut manifest,
            audio_ttl_secs,
        )
        .await
    }

    async fn sign_scene_audio(&self, manifest: &mut ChapterManifest, secs: u64) {
        for scene in &mut manifest.scenes {
            let Some(key) = scene.depth.audio_key.clone() else {
                continue;
            };
            scene.depth.audio_url = match self.storage.get_presigned_url(&key, secs).await {
                Ok(url) => Some(url),
                Err(e) => {
                    tracing::warn!(
                        error = format!("{e:#}"),
                        key,
                        "Falling back to public audio URL"
                    );
                    Some(self.storage.get_public_url(&key))
                }
            };
        }
    }

    /// Prefers the persisted page extract (paragraphs + citations later);
    /// falls back to re-reading the original file.
    async fn chapter_text_for_generation(
        &self,
        subject: &Subject,
        chapter: &Chapter,
    ) -> Result<String> {
        let (text, _) = self.chapter_source_for_generation(subject, chapter).await?;
        Ok(text)
    }

    async fn chapter_source_for_generation(
        &self,
        subject: &Subject,
        chapter: &Chapter,
    ) -> Result<(String, Vec<ExtractedPage>)> {
        if let Some(prefix) = subject.extract_prefix.as_deref()
            && let Some(pages) = self
                .chapter_pages_from_extract(prefix, chapter, subject.total_pages)
                .await?
        {
            return Ok((prompt_from_pages(&pages), pages));
        }

        let file_data = self
            .storage
            .download_file(&subject.file_path)
            .await
            .context("Failed to download file")?;
        let extension = subject.file_path.split('.').next_back().unwrap_or("");

        if extension.eq_ignore_ascii_case("pdf") {
            let doc =
                lopdf::Document::load_mem(&file_data).context("Failed to load PDF document")?;
            let total_pages = doc.get_pages().len() as i32;
            let (start, end) = chapter_page_window(chapter, total_pages);
            let page_numbers: Vec<u32> = (start as u32..=end as u32).collect();
            let text = doc
                .extract_text(&page_numbers)
                .context("Failed to extract text from PDF chapter")?;
            Ok((text, Vec::new()))
        } else {
            let text = self.extract_text(&file_data, extension).await?;
            Ok((text, Vec::new()))
        }
    }

    async fn chapter_pages_from_extract(
        &self,
        prefix: &str,
        chapter: &Chapter,
        total_pages: Option<i32>,
    ) -> Result<Option<Vec<ExtractedPage>>> {
        let total = match total_pages {
            Some(t) if t > 0 => t,
            _ => {
                let bytes = match self
                    .storage
                    .download_file(&manifest_object_key(prefix))
                    .await
                {
                    Ok(b) => b,
                    Err(_) => return Ok(None),
                };
                let manifest: ExtractManifest = serde_json::from_slice(&bytes)?;
                manifest.total_pages as i32
            }
        };

        let (start, end) = chapter_page_window(chapter, total);
        let mut pages = Vec::new();
        for page in start as u32..=end as u32 {
            let key = page_object_key(prefix, page);
            let bytes = match self.storage.download_file(&key).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(error = format!("{e:#}"), key, "Missing extract page");
                    continue;
                }
            };
            pages.push(serde_json::from_slice::<ExtractedPage>(&bytes)?);
        }

        if pages.is_empty() {
            return Ok(None);
        }
        Ok(Some(pages))
    }
}

fn chapter_page_window(chapter: &Chapter, total_pages: i32) -> (i32, i32) {
    let start = chapter.page_start.clamp(1, total_pages.max(1));
    let end = if chapter.page_end < start {
        tracing::warn!(
            page_start = chapter.page_start,
            page_end = chapter.page_end,
            "Chapter has an invalid page range; falling back to a 30-page window"
        );
        (start + 29).min(total_pages.max(start))
    } else {
        chapter.page_end.min(total_pages.max(start))
    };
    (start, end)
}

// Unit tests for private helpers live here; the `Job` wire-format contract is
// tested in tests/job_format_test.rs.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_returns_short_strings_unchanged() {
        assert_eq!(truncate_to_char_boundary("hello", 10), "hello");
        assert_eq!(truncate_to_char_boundary("hello", 5), "hello");
        assert_eq!(truncate_to_char_boundary("", 10), "");
    }

    #[test]
    fn truncate_cuts_ascii_at_exact_limit() {
        assert_eq!(truncate_to_char_boundary("hello world", 5), "hello");
    }

    #[test]
    fn truncate_never_splits_multibyte_chars() {
        // 'é' is 2 bytes in UTF-8; a naive `&s[..3]` would panic here.
        let s = "ééé"; // 6 bytes
        assert_eq!(truncate_to_char_boundary(s, 3), "é");
        assert_eq!(truncate_to_char_boundary(s, 4), "éé");

        // 4-byte emoji
        let s = "a📚b"; // 1 + 4 + 1 bytes
        assert_eq!(truncate_to_char_boundary(s, 2), "a");
        assert_eq!(truncate_to_char_boundary(s, 5), "a📚");
    }

    #[test]
    fn truncate_result_is_always_within_limit() {
        let s = "日本語のテキスト";
        for max in 0..=s.len() {
            let t = truncate_to_char_boundary(s, max);
            assert!(t.len() <= max);
            assert!(s.starts_with(t));
        }
    }

    #[test]
    fn chapter_page_window_falls_back_when_range_is_invalid() {
        let chapter = Chapter {
            id: "c".into(),
            subject_id: "s".into(),
            user_id: "u".into(),
            title: "T".into(),
            page_start: 0,
            page_end: 0,
            order_index: 0,
            processing_status: None,
            processed_slides: None,
            total_slides: None,
            job_id: None,
            package_key: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let (start, end) = chapter_page_window(&chapter, 100);
        assert_eq!(start, 1);
        assert_eq!(end, 30);
    }
}
