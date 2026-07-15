use crate::config::Settings;
use crate::db::Database;
use crate::models::{Chapter, Slide};
use crate::services::{AIService, StorageService};
use anyhow::{Context, Result};
use chrono::Utc;
use dotext::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::Arc;
use uuid::Uuid;

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
    },
    GenerateSlides {
        user_id: String,
        subject_id: String,
        chapter_id: String,
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
}

#[derive(Clone)]
pub struct BackgroundTasksService {
    db: Arc<Database>,
    storage: Arc<StorageService>,
    ai: Arc<AIService>,
    queue: Option<JobQueue>,
}

impl BackgroundTasksService {
    pub fn new(
        db: Arc<Database>,
        storage: Arc<StorageService>,
        ai: Arc<AIService>,
        queue: Option<JobQueue>,
    ) -> Self {
        Self {
            db,
            storage,
            ai,
            queue,
        }
    }

    /// Hands a job off for background execution: to SQS when a queue is
    /// configured (required on Lambda, where in-process tasks stall once the
    /// response is sent), otherwise to an in-process tokio task.
    pub async fn dispatch(&self, job: Job) -> Result<()> {
        match &self.queue {
            Some(queue) => {
                tracing::info!(?job, "Dispatching job to SQS");
                queue.send(&job).await
            }
            None => {
                tracing::info!(?job, "Running job in-process");
                let service = self.clone();
                tokio::spawn(async move {
                    // Failures are logged and persisted as "failed" status inside run_job.
                    let _ = service.run_job(job).await;
                });
                Ok(())
            }
        }
    }

    /// Executes a job to completion. On failure the related entity is marked
    /// "failed" and the error is returned so SQS consumers can retry.
    pub async fn run_job(&self, job: Job) -> Result<()> {
        match job {
            Job::ProcessSubject {
                user_id,
                subject_id,
                file_s3path,
            } => {
                self.process_subject_bg(user_id, subject_id, file_s3path)
                    .await
            }
            Job::GenerateSlides {
                user_id,
                subject_id,
                chapter_id,
            } => {
                let chapter = self
                    .db
                    .get_chapter(&subject_id, &chapter_id)
                    .await?
                    .context("Chapter not found")?;
                self.generate_slides_bg(user_id, subject_id, chapter).await
            }
        }
    }

    pub async fn process_subject_bg(
        &self,
        user_id: String,
        subject_id: String,
        file_s3path: String,
    ) -> Result<()> {
        tracing::info!(
            user_id = %user_id,
            subject_id = %subject_id,
            file_s3path = %file_s3path,
            "Background: processing subject file"
        );

        if let Err(e) = self
            .do_process_subject(&user_id, &subject_id, &file_s3path)
            .await
        {
            tracing::error!(
                error = format!("{e:#}"),
                "Failed to process subject background task"
            );
            if let Ok(Some(mut subject)) = self.db.get_subject(&subject_id).await {
                subject.processing_status = "failed".to_string();
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
                let pages = doc.get_pages();
                // Heuristic: Extract first 20 pages if PDF is large
                let pages_to_extract = std::cmp::min(20, pages.len());
                let mut page_numbers = Vec::new();
                for i in 1..=pages_to_extract {
                    page_numbers.push(i as u32);
                }
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

    /// Extracts text from the first pages of a PDF one page at a time,
    /// persisting processed/total page counts on the subject so the UI can
    /// show real progress. Pages that fail to extract are skipped.
    async fn extract_pdf_text_with_progress(
        &self,
        file_data: &[u8],
        subject_id: &str,
    ) -> Result<String> {
        let doc = lopdf::Document::load_mem(file_data).context("Failed to load PDF document")?;
        // Heuristic: Extract first 20 pages if PDF is large
        let total = std::cmp::min(20, doc.get_pages().len()) as i32;
        self.update_page_progress(subject_id, 0, total).await;

        let mut text = String::new();
        for page in 1..=total {
            match doc.extract_text(&[page as u32]) {
                Ok(page_text) => {
                    text.push_str(&page_text);
                    text.push('\n');
                }
                Err(e) => {
                    tracing::warn!(
                        error = format!("{e:#}"),
                        page,
                        "Skipping unextractable page"
                    );
                }
            }
            self.update_page_progress(subject_id, page, total).await;
        }

        if text.trim().is_empty() {
            anyhow::bail!("Failed to extract text from any of the first {total} PDF pages");
        }
        Ok(text)
    }

    /// Best-effort progress write: failures are ignored so a flaky status
    /// update can never abort processing itself.
    async fn update_page_progress(&self, subject_id: &str, processed: i32, total: i32) {
        if let Ok(Some(mut subject)) = self.db.get_subject(subject_id).await {
            subject.processed_pages = Some(processed);
            subject.total_pages = Some(total);
            subject.updated_at = Utc::now();
            let _ = self.db.save_subject(&subject).await;
        }
    }

    /// Best-effort slide-count progress write for a chapter's generation run.
    async fn update_slide_progress(
        &self,
        subject_id: &str,
        chapter_id: &str,
        processed: i32,
        total: i32,
    ) {
        if let Ok(Some(mut chapter)) = self.db.get_chapter(subject_id, chapter_id).await {
            chapter.processed_slides = Some(processed);
            chapter.total_slides = Some(total);
            chapter.updated_at = Utc::now();
            let _ = self.db.save_chapter(&chapter).await;
        }
    }

    async fn do_process_subject(
        &self,
        user_id: &str,
        subject_id: &str,
        file_s3path: &str,
    ) -> Result<()> {
        // 1. Download file from S3
        let file_data = self
            .storage
            .download_file(file_s3path)
            .await
            .context("Failed to download file from S3")?;

        // 2. Extract text based on extension, reporting page progress for PDFs
        let extension = file_s3path.split('.').next_back().unwrap_or("");
        let toc_text = if extension.eq_ignore_ascii_case("pdf") {
            self.extract_pdf_text_with_progress(&file_data, subject_id)
                .await?
        } else {
            self.extract_text(&file_data, extension).await?
        };

        // 3. Call AI to analyze TOC
        // AI can handle a reasonably large amount of text, but we may want to truncate if too large
        let truncated_toc = truncate_to_char_boundary(&toc_text, 10000);

        let chapters_data = self
            .ai
            .analyze_book_toc(truncated_toc)
            .await
            .context("AI failed to analyze TOC")?;

        // 4. Create chapters in DynamoDB
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
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }
        self.db
            .save_chapters(chapters)
            .await
            .context("Failed to save chapters to DB")?;

        // 5. Update subject processing status to "completed"
        if let Some(mut subject) = self.db.get_subject(subject_id).await? {
            subject.processing_status = "completed".to_string();
            subject.updated_at = Utc::now();
            self.db
                .save_subject(&subject)
                .await
                .context("Failed to update subject status")?;
        }

        Ok(())
    }

    pub async fn generate_slides_bg(
        &self,
        user_id: String,
        subject_id: String,
        chapter: Chapter,
    ) -> Result<()> {
        tracing::info!(
            user_id = %user_id,
            subject_id = %subject_id,
            chapter_id = %chapter.id,
            "Background: generating slides for chapter"
        );

        if let Err(e) = self
            .do_generate_slides(&user_id, &subject_id, &chapter)
            .await
        {
            tracing::error!(
                error = format!("{e:#}"),
                "Failed to generate slides background task"
            );
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
    ) -> Result<()> {
        // 1. Get subject to find file path
        let subject = self
            .db
            .get_subject(subject_id)
            .await?
            .context("Subject not found")?;

        // 2. Download PDF/DOCX
        let file_data = self
            .storage
            .download_file(&subject.file_path)
            .await
            .context("Failed to download file")?;

        let extension = subject.file_path.split('.').next_back().unwrap_or("");

        // 3. Extract text
        let chapter_text = if extension.to_lowercase() == "pdf" {
            let doc =
                lopdf::Document::load_mem(&file_data).context("Failed to load PDF document")?;
            let total_pages = doc.get_pages().len() as i32;
            let start = chapter.page_start.clamp(1, total_pages);
            let end = if chapter.page_end < start {
                // TOC analysis returned no usable range (e.g. 0/0): fall back
                // to a 20-page window so generation still has content.
                tracing::warn!(
                    page_start = chapter.page_start,
                    page_end = chapter.page_end,
                    "Chapter has an invalid page range; falling back to a 20-page window"
                );
                (start + 19).min(total_pages)
            } else {
                chapter.page_end.min(total_pages)
            };

            let mut page_numbers = Vec::new();
            for i in start..=end {
                page_numbers.push(i as u32);
            }
            doc.extract_text(&page_numbers)
                .context("Failed to extract text from PDF chapter")?
        } else {
            // For DOCX/DOC, we just extract all and let AI pick (better to split by paragraphs if large)
            // But usually chapters are within reasonable limits for LLM context if it's a doc
            self.extract_text(&file_data, extension).await?
        };

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
        self.update_slide_progress(subject_id, &chapter.id, 0, total_slides)
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

            self.update_slide_progress(subject_id, &chapter.id, idx as i32 + 1, total_slides)
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

        Ok(())
    }
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
}
