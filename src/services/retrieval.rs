//! Chapter retrieval: Gemini embeddings into optional Qdrant, with a
//! lexical fallback over the page extract. Ask and MCP share this path
//! so every result carries a citation (page + paragraph), never a bare
//! paragraph.

use crate::db::Database;
use crate::models::{Chapter, ChapterManifest, Citation, SceneSpec, Subject, chapter_manifest_key};
use crate::services::ai::{AIService, EmbedTask};
use crate::services::citations::{ParagraphIndex, first_quotable_span, retrieve_paragraphs};
use crate::services::extract::{
    ExtractManifest, ExtractedPage, manifest_object_key, page_object_key,
    pdf_page_from_paragraph_id,
};
use crate::services::qdrant::{QdrantClient, QdrantPoint};
use crate::services::storage::StorageService;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

const DEFAULT_LIMIT: usize = 8;
const POINT_NS: Uuid = uuid::uuid!("6ba7b810-9dad-11d1-80b4-00c04fd430c8");

#[derive(Clone)]
pub struct RetrievalService {
    db: Arc<Database>,
    storage: Arc<StorageService>,
    ai: Arc<AIService>,
    qdrant: Option<QdrantClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchHit {
    pub book_id: String,
    pub chapter_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
    pub paragraph_id: String,
    pub pdf_page: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printed_page: Option<i32>,
    pub quote: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AskRequest {
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AskResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
}

impl RetrievalService {
    pub fn new(
        db: Arc<Database>,
        storage: Arc<StorageService>,
        ai: Arc<AIService>,
        qdrant: Option<QdrantClient>,
    ) -> Self {
        Self {
            db,
            storage,
            ai,
            qdrant,
        }
    }

    pub async fn index_chapter(
        &self,
        tenant_id: Option<&str>,
        subject: &Subject,
        chapter: &Chapter,
        pages: &[ExtractedPage],
        scenes: &[SceneSpec],
    ) -> Result<usize> {
        let Some(qdrant) = &self.qdrant else {
            return Ok(0);
        };
        if pages.is_empty() {
            return Ok(0);
        }

        let scene_by_para = scene_ids_by_paragraph(scenes);
        let mut texts = Vec::new();
        let mut meta = Vec::new();
        for page in pages {
            for para in &page.paragraphs {
                let text = para.text.trim();
                if text.is_empty() {
                    continue;
                }
                texts.push(text.to_string());
                meta.push((page, para));
            }
        }
        if texts.is_empty() {
            return Ok(0);
        }

        let vectors = self.ai.embed_texts(&texts, EmbedTask::Document).await?;
        if vectors.len() != texts.len() {
            anyhow::bail!("embedding count does not match paragraph count");
        }

        let mut points = Vec::with_capacity(texts.len());
        for ((page, para), vector) in meta.into_iter().zip(vectors) {
            let scene_ids = scene_by_para.get(&para.id).cloned().unwrap_or_default();
            points.push(QdrantPoint {
                id: point_id(&subject.id, &para.id),
                vector,
                payload: serde_json::json!({
                    "tenant_id": tenant_id,
                    "book_id": subject.id,
                    "chapter_id": chapter.id,
                    "scene_ids": scene_ids,
                    "pdf_page": page.pdf_page as i32,
                    "printed_page": page.printed_page,
                    "paragraph_id": para.id,
                    "text": para.text,
                }),
            });
        }

        let count = points.len();
        qdrant.upsert(&points).await?;
        Ok(count)
    }

    pub async fn search(
        &self,
        subject: &Subject,
        chapter: Option<&Chapter>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let limit = limit.max(1);
        if let Some(qdrant) = &self.qdrant {
            match self
                .vector_search(qdrant, subject, chapter, query, limit)
                .await
            {
                Ok(hits) if !hits.is_empty() => return Ok(hits),
                Ok(_) => {}
                Err(e) => tracing::warn!(
                    error = format!("{e:#}"),
                    book_id = %subject.id,
                    "Vector search failed; using lexical fallback"
                ),
            }
        }
        self.lexical_search(subject, chapter, query, limit).await
    }

    pub async fn ask(
        &self,
        subject: &Subject,
        chapter: Option<&Chapter>,
        question: &str,
    ) -> Result<AskResponse> {
        let hits = self
            .search(subject, chapter, question, DEFAULT_LIMIT)
            .await?;
        if hits.is_empty() {
            return Ok(AskResponse {
                answer: "I could not find a cited passage that answers this.".to_string(),
                citations: Vec::new(),
            });
        }
        let sources = format_sources(&hits);
        let grounded = self.ai.answer_from_citations(question, &sources).await?;
        let citations = citations_for_answer(&hits, &grounded.citation_ids);
        Ok(AskResponse {
            answer: grounded.answer,
            citations,
        })
    }

    pub async fn get_scene(
        &self,
        subject: &Subject,
        chapter_id: &str,
        scene_id: &str,
    ) -> Result<Option<SceneSpec>> {
        let manifest = match self
            .load_manifest(&subject.user_id, &subject.id, chapter_id)
            .await
        {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };
        Ok(manifest
            .scenes
            .into_iter()
            .find(|s| s.id == scene_id)
            .map(|mut s| {
                s.depth.audio_url = None;
                s
            }))
    }

    pub async fn get_citation(
        &self,
        subject: &Subject,
        paragraph_id: &str,
    ) -> Result<Option<SearchHit>> {
        let Some(page_no) = pdf_page_from_paragraph_id(paragraph_id) else {
            return Ok(None);
        };
        let prefix = subject.extract_prefix.clone().unwrap_or_else(|| {
            crate::services::extract::extract_prefix(&subject.user_id, &subject.id)
        });
        let key = page_object_key(&prefix, page_no);
        let bytes = match self.storage.download_file(&key).await {
            Ok(b) => b,
            Err(_) => return Ok(None),
        };
        let page: ExtractedPage = serde_json::from_slice(&bytes)?;
        let Some(para) = page.paragraphs.iter().find(|p| p.id == paragraph_id) else {
            return Ok(None);
        };
        let chapter_id = self
            .chapter_id_for_page(&subject.id, page.pdf_page as i32)
            .await
            .unwrap_or_default();
        Ok(Some(hit_from_paragraph(
            &subject.id,
            &chapter_id,
            None,
            page.pdf_page as i32,
            page.printed_page,
            &para.id,
            &para.text,
            1.0,
        )))
    }

    async fn vector_search(
        &self,
        qdrant: &QdrantClient,
        subject: &Subject,
        chapter: Option<&Chapter>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let vectors = self
            .ai
            .embed_texts(&[query.to_string()], EmbedTask::Query)
            .await?;
        let Some(vector) = vectors.into_iter().next() else {
            return Ok(Vec::new());
        };
        let raw = qdrant
            .search(&vector, &subject.id, chapter.map(|c| c.id.as_str()), limit)
            .await?;
        Ok(raw
            .into_iter()
            .filter_map(|h| hit_from_payload(&subject.id, h.score, h.payload))
            .collect())
    }

    async fn lexical_search(
        &self,
        subject: &Subject,
        chapter: Option<&Chapter>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let pages = match chapter {
            Some(ch) => self.load_chapter_pages(subject, ch).await?,
            None => self.load_book_pages(subject).await?,
        };
        if pages.is_empty() {
            return Ok(Vec::new());
        }
        let index = ParagraphIndex::from_pages(&pages);
        let chapter_id = chapter.map(|c| c.id.clone()).unwrap_or_default();
        let hits = retrieve_paragraphs(query, &index, limit)
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                hit_from_paragraph(
                    &subject.id,
                    &chapter_id,
                    None,
                    p.pdf_page,
                    p.printed_page,
                    &p.id,
                    &p.text,
                    (limit - i) as f32,
                )
            })
            .collect();
        Ok(hits)
    }

    async fn load_chapter_pages(
        &self,
        subject: &Subject,
        chapter: &Chapter,
    ) -> Result<Vec<ExtractedPage>> {
        let prefix = match subject.extract_prefix.as_deref() {
            Some(p) => p.to_string(),
            None => return Ok(Vec::new()),
        };
        let total = subject.total_pages.unwrap_or(chapter.page_end);
        let start = chapter.page_start.max(1) as u32;
        let end = chapter.page_end.min(total.max(chapter.page_end)) as u32;
        self.load_page_range(&prefix, start, end).await
    }

    async fn load_book_pages(&self, subject: &Subject) -> Result<Vec<ExtractedPage>> {
        let prefix = match subject.extract_prefix.as_deref() {
            Some(p) => p.to_string(),
            None => return Ok(Vec::new()),
        };
        let total = match subject.total_pages {
            Some(t) if t > 0 => t as u32,
            _ => {
                let bytes = self
                    .storage
                    .download_file(&manifest_object_key(&prefix))
                    .await
                    .context("extract manifest missing")?;
                let manifest: ExtractManifest = serde_json::from_slice(&bytes)?;
                manifest.total_pages
            }
        };
        if total == 0 {
            return Ok(Vec::new());
        }
        self.load_page_range(&prefix, 1, total).await
    }

    async fn load_page_range(
        &self,
        prefix: &str,
        start: u32,
        end: u32,
    ) -> Result<Vec<ExtractedPage>> {
        let mut pages = Vec::new();
        for page in start..=end {
            let key = page_object_key(prefix, page);
            match self.storage.download_file(&key).await {
                Ok(bytes) => pages.push(serde_json::from_slice(&bytes)?),
                Err(_) => continue,
            }
        }
        Ok(pages)
    }

    async fn load_manifest(
        &self,
        user_id: &str,
        subject_id: &str,
        chapter_id: &str,
    ) -> Result<ChapterManifest> {
        let key = chapter_manifest_key(user_id, subject_id, chapter_id);
        let bytes = self.storage.download_file(&key).await?;
        serde_json::from_slice(&bytes).context("Failed to parse chapter manifest")
    }

    async fn chapter_id_for_page(&self, subject_id: &str, pdf_page: i32) -> Option<String> {
        let chapters = self.db.get_chapters_by_subject(subject_id).await.ok()?;
        chapters
            .into_iter()
            .find(|c| pdf_page >= c.page_start && pdf_page <= c.page_end)
            .map(|c| c.id)
    }
}

pub fn point_id(book_id: &str, paragraph_id: &str) -> String {
    Uuid::new_v5(&POINT_NS, format!("{book_id}:{paragraph_id}").as_bytes()).to_string()
}

fn scene_ids_by_paragraph(scenes: &[SceneSpec]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for scene in scenes {
        for cite in &scene.citations {
            let entry = map.entry(cite.paragraph_id.clone()).or_default();
            if !entry.contains(&scene.id) {
                entry.push(scene.id.clone());
            }
        }
    }
    map
}

#[allow(clippy::too_many_arguments)]
fn hit_from_paragraph(
    book_id: &str,
    chapter_id: &str,
    scene_id: Option<String>,
    pdf_page: i32,
    printed_page: Option<i32>,
    paragraph_id: &str,
    text: &str,
    score: f32,
) -> SearchHit {
    SearchHit {
        book_id: book_id.to_string(),
        chapter_id: chapter_id.to_string(),
        scene_id,
        paragraph_id: paragraph_id.to_string(),
        pdf_page,
        printed_page,
        quote: first_quotable_span(text).unwrap_or_else(|| text.chars().take(220).collect()),
        score,
    }
}

fn hit_from_payload(book_id: &str, score: f32, payload: serde_json::Value) -> Option<SearchHit> {
    let paragraph_id = payload.get("paragraph_id")?.as_str()?.to_string();
    let pdf_page = payload.get("pdf_page")?.as_i64()? as i32;
    let printed_page = payload
        .get("printed_page")
        .and_then(|v| v.as_i64())
        .map(|n| n as i32);
    let chapter_id = payload
        .get("chapter_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let scene_id = payload
        .get("scene_ids")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let text = payload.get("text").and_then(|v| v.as_str()).unwrap_or("");
    Some(hit_from_paragraph(
        book_id,
        &chapter_id,
        scene_id,
        pdf_page,
        printed_page,
        &paragraph_id,
        text,
        score,
    ))
}

fn format_sources(hits: &[SearchHit]) -> String {
    hits.iter()
        .map(|h| {
            let printed = match h.printed_page {
                Some(n) => format!("printed p.{n}"),
                None => "printed unknown".into(),
            };
            let scene = h
                .scene_id
                .as_deref()
                .map(|s| format!(" scene:{s}"))
                .unwrap_or_default();
            format!(
                "[{}] PDF p.{} / {}{scene}\nQuote: {}",
                h.paragraph_id, h.pdf_page, printed, h.quote
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Keep only citations the model named that we actually retrieved. If it
/// named none (or only invented ids), return every hit so the student still
/// sees real page/paragraph evidence.
pub fn citations_for_answer(hits: &[SearchHit], citation_ids: &[String]) -> Vec<Citation> {
    let allowed: HashSet<&str> = hits.iter().map(|h| h.paragraph_id.as_str()).collect();
    let named: Vec<Citation> = citation_ids
        .iter()
        .filter(|id| allowed.contains(id.as_str()))
        .filter_map(|id| {
            hits.iter()
                .find(|h| h.paragraph_id == *id)
                .map(hit_to_citation)
        })
        .collect();
    if named.is_empty() {
        hits.iter().map(hit_to_citation).collect()
    } else {
        named
    }
}

fn hit_to_citation(hit: &SearchHit) -> Citation {
    Citation {
        pdf_page: hit.pdf_page,
        printed_page: hit.printed_page,
        paragraph_id: hit.paragraph_id.clone(),
        quote: hit.quote.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str, page: i32) -> SearchHit {
        SearchHit {
            book_id: "b".into(),
            chapter_id: "c".into(),
            scene_id: Some("packets".into()),
            paragraph_id: id.into(),
            pdf_page: page,
            printed_page: Some(page),
            quote: format!("quote for {id}"),
            score: 1.0,
        }
    }

    #[test]
    fn point_ids_are_stable() {
        assert_eq!(point_id("book", "p12-2"), point_id("book", "p12-2"));
        assert_ne!(point_id("book", "p12-2"), point_id("book", "p12-3"));
    }

    #[test]
    fn drops_invented_citation_ids() {
        let hits = vec![hit("p12-2", 12), hit("p13-1", 13)];
        let cites = citations_for_answer(&hits, &["p12-2".into(), "p99-9".into()]);
        assert_eq!(cites.len(), 1);
        assert_eq!(cites[0].paragraph_id, "p12-2");
        assert_eq!(cites[0].pdf_page, 12);
    }

    #[test]
    fn falls_back_to_all_hits_when_model_cites_nothing() {
        let hits = vec![hit("p12-2", 12)];
        let cites = citations_for_answer(&hits, &["invented".into()]);
        assert_eq!(cites.len(), 1);
        assert_eq!(cites[0].paragraph_id, "p12-2");
    }

    #[test]
    fn every_hit_has_a_quote() {
        let h = hit_from_paragraph(
            "b",
            "c",
            None,
            4,
            Some(2),
            "p4-1",
            "A packet is a box.",
            1.0,
        );
        assert!(!h.quote.is_empty());
        assert_eq!(h.paragraph_id, "p4-1");
    }
}
