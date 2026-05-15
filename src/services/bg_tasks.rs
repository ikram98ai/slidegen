use crate::db::Database;
use crate::models::{Chapter, Slide};
use crate::services::{AIService, StorageService};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use std::io::{Read, Write};
use std::sync::Arc;
use uuid::Uuid;
use dotext::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct BackgroundTasksService {
    db: Arc<Database>,
    storage: Arc<StorageService>,
    ai: Arc<AIService>,
}

impl BackgroundTasksService {
    pub fn new(db: Arc<Database>, storage: Arc<StorageService>, ai: Arc<AIService>) -> Self {
        Self { db, storage, ai }
    }

    pub async fn process_subject_bg(
        &self,
        user_id: String,
        subject_id: String,
        file_s3path: String,
    ) {
        tracing::info!(
            user_id = %user_id,
            subject_id = %subject_id,
            file_s3path = %file_s3path,
            "Background: processing subject file"
        );

        if let Err(e) = self.do_process_subject(&user_id, &subject_id, &file_s3path).await {
            tracing::error!(error = %e, "Failed to process subject background task");
            // Optionally update subject status to "failed"
            if let Ok(Some(mut subject)) = self.db.get_subject(&subject_id).await {
                subject.processing_status = "failed".to_string();
                let _ = self.db.save_subject(&subject).await;
            }
        }
    }

    async fn extract_text(&self, file_data: &[u8], extension: &str) -> Result<String> {
        match extension.to_lowercase().as_str() {
            "pdf" => {
                let doc = lopdf::Document::load_mem(file_data)
                    .context("Failed to load PDF document")?;
                let pages = doc.get_pages();
                // Heuristic: Extract first 20 pages if PDF is large
                let pages_to_extract = std::cmp::min(20, pages.len());
                let mut page_numbers = Vec::new();
                for i in 1..=pages_to_extract {
                    page_numbers.push(i as u32);
                }
                doc.extract_text(&page_numbers).context("Failed to extract PDF text")
            }
            "docx" => {
                let mut tmpfile = tempfile::NamedTempFile::new()
                    .context("Failed to create temporary file for DOCX")?;
                tmpfile.write_all(file_data).context("Failed to write DOCX data to temp file")?;
                
                let mut reader = Docx::open(tmpfile.path())
                    .map_err(|e| anyhow::anyhow!("Failed to open DOCX: {:?}", e))?;
                
                let mut text = String::new();
                reader.read_to_string(&mut text).context("Failed to read text from DOCX")?;
                Ok(text)
            }
            _ => anyhow::bail!("Unsupported file extension: {}", extension),
        }
    }

    async fn do_process_subject(&self, user_id: &str, subject_id: &str, file_s3path: &str) -> Result<()> {
        // 1. Download file from S3
        let file_data = self.storage.download_file(file_s3path).await
            .context("Failed to download file from S3")?;

        // 2. Extract text based on extension
        let extension = file_s3path.split('.').last().unwrap_or("");
        let toc_text = self.extract_text(&file_data, extension).await?;

        // 3. Call AI to analyze TOC
        // AI can handle a reasonably large amount of text, but we may want to truncate if too large
        let truncated_toc = if toc_text.len() > 10000 {
            &toc_text[..10000]
        } else {
            &toc_text
        };

        let chapters_data = self.ai.analyze_book_toc(truncated_toc).await
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
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }
        self.db.save_chapters(chapters).await.context("Failed to save chapters to DB")?;

        // 5. Update subject processing status to "completed"
        if let Some(mut subject) = self.db.get_subject(subject_id).await? {
            subject.processing_status = "completed".to_string();
            subject.updated_at = Utc::now();
            self.db.save_subject(&subject).await.context("Failed to update subject status")?;
        }

        Ok(())
    }

    pub async fn generate_slides_bg(
        &self,
        user_id: String,
        subject_id: String,
        chapter: Chapter,
    ) {
        tracing::info!(
            user_id = %user_id,
            subject_id = %subject_id,
            chapter_id = %chapter.id,
            "Background: generating slides for chapter"
        );

        if let Err(e) = self.do_generate_slides(&user_id, &subject_id, &chapter).await {
            tracing::error!(error = %e, "Failed to generate slides background task");
            if let Ok(Some(mut c)) = self.db.get_chapter(&subject_id, &chapter.id).await {
                c.processing_status = Some("failed".to_string());
                let _ = self.db.save_chapter(&c).await;
            }
        }
    }

    async fn do_generate_slides(&self, user_id: &str, subject_id: &str, chapter: &Chapter) -> Result<()> {
        // 1. Get subject to find file path
        let subject = self.db.get_subject(subject_id).await?
            .context("Subject not found")?;

        // 2. Download PDF/DOCX
        let file_data = self.storage.download_file(&subject.file_path).await
            .context("Failed to download file")?;

        let extension = subject.file_path.split('.').last().unwrap_or("");
        
        // 3. Extract text
        let chapter_text = if extension.to_lowercase() == "pdf" {
            let doc = lopdf::Document::load_mem(&file_data)
                .context("Failed to load PDF document")?;
            let total_pages = doc.get_pages().len() as i32;
            let start = std::cmp::max(1, chapter.page_start);
            let end = std::cmp::min(total_pages, chapter.page_end);

            let mut page_numbers = Vec::new();
            for i in start..=end {
                page_numbers.push(i as u32);
            }
            doc.extract_text(&page_numbers).context("Failed to extract text from PDF chapter")?
        } else {
            // For DOCX/DOC, we just extract all and let AI pick (better to split by paragraphs if large)
            // But usually chapters are within reasonable limits for LLM context if it's a doc
            self.extract_text(&file_data, extension).await?
        };

        if chapter_text.trim().is_empty() {
            anyhow::bail!("No text extracted for chapter in {}", subject.file_path);
        }

        // 4. Call AI to generate slides
        let slides_data = self.ai.generate_slides(&chapter_text).await
            .context("AI failed to generate slides")?;

        // 5. Save slides and generate audio
        for (idx, s_data) in slides_data.into_iter().enumerate() {
            let slide_id = Uuid::new_v4().to_string();
            let mut voice_url = None;

            // Generate Audio
            let audio_text = format!("{}. {}", s_data.title, s_data.explanation);
            if let Ok(audio_base64) = self.ai.generate_slide_audio(&audio_text).await {
                if let Ok(audio_bytes) = general_purpose::STANDARD.decode(audio_base64) {
                    let audio_key = format!("audio/{}/{}/{}.mp3", user_id, chapter.id, slide_id);
                    if self.storage.upload_file(&audio_key, audio_bytes).await.is_ok() {
                        voice_url = Some(audio_key);
                    }
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

            self.db.save_slide(&slide).await.context("Failed to save slide")?;
        }

        if let Some(mut c) = self.db.get_chapter(subject_id, &chapter.id).await? {
            c.processing_status = Some("completed".to_string());
            c.updated_at = Utc::now();
            self.db.save_chapter(&c).await.context("Failed to update chapter status")?;
        }

        Ok(())
    }
}
