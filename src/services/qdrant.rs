//! Optional Qdrant REST client. When `QDRANT_URL` is unset the Ask path
//! uses lexical retrieval instead.

use crate::config::Settings;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug)]
pub struct QdrantClient {
    http: Client,
    base_url: String,
    api_key: Option<String>,
    collection: String,
    dimensions: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantPoint {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct QdrantHit {
    pub score: f32,
    pub payload: serde_json::Value,
}

impl QdrantClient {
    pub fn from_settings(settings: &Settings) -> Option<Self> {
        let base_url = settings.qdrant_url.clone()?;
        Some(Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: settings.qdrant_api_key.clone(),
            collection: settings.qdrant_collection.clone(),
            dimensions: settings.embedding_dimensions,
        })
    }

    pub fn collection(&self) -> &str {
        &self.collection
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => req.header("api-key", key),
            None => req,
        }
    }

    pub async fn ensure_collection(&self) -> Result<()> {
        let url = format!("{}/collections/{}", self.base_url, self.collection);
        let head = self.apply_auth(self.http.get(&url)).send().await?;
        if head.status().is_success() {
            return Ok(());
        }
        let body = json!({
            "vectors": { "size": self.dimensions, "distance": "Cosine" }
        });
        let resp = self
            .apply_auth(self.http.put(&url))
            .json(&body)
            .send()
            .await
            .context("Failed to create Qdrant collection")?;
        if resp.status().is_success() || resp.status().as_u16() == 409 {
            return Ok(());
        }
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Qdrant create collection failed ({status}): {text}");
    }

    pub async fn upsert(&self, points: &[QdrantPoint]) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }
        self.ensure_collection().await?;
        let url = format!(
            "{}/collections/{}/points?wait=true",
            self.base_url, self.collection
        );
        let body = json!({ "points": points });
        let resp = self
            .apply_auth(self.http.put(&url))
            .json(&body)
            .send()
            .await
            .context("Failed to upsert Qdrant points")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant upsert failed ({status}): {text}");
        }
        Ok(())
    }

    pub async fn search(
        &self,
        vector: &[f32],
        book_id: &str,
        chapter_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<QdrantHit>> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.base_url, self.collection
        );
        let body = json!({
            "vector": vector,
            "limit": limit,
            "with_payload": true,
            "filter": search_filter(book_id, chapter_id),
        });
        let resp = self
            .apply_auth(self.http.post(&url))
            .json(&body)
            .send()
            .await
            .context("Failed to search Qdrant")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Qdrant search failed ({status}): {text}");
        }
        let parsed: QdrantSearchResponse = resp.json().await?;
        Ok(parsed
            .result
            .into_iter()
            .map(|r| QdrantHit {
                score: r.score,
                payload: r.payload.unwrap_or(serde_json::Value::Null),
            })
            .collect())
    }
}

pub fn search_filter(book_id: &str, chapter_id: Option<&str>) -> serde_json::Value {
    let mut must = vec![json!({ "key": "book_id", "match": { "value": book_id } })];
    if let Some(chapter_id) = chapter_id {
        must.push(json!({ "key": "chapter_id", "match": { "value": chapter_id } }));
    }
    json!({ "must": must })
}

#[derive(Deserialize)]
struct QdrantSearchResponse {
    #[serde(default)]
    result: Vec<QdrantSearchResult>,
}

#[derive(Deserialize)]
struct QdrantSearchResult {
    score: f32,
    payload: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_includes_optional_chapter() {
        let book_only = search_filter("b1", None);
        assert_eq!(book_only["must"].as_array().unwrap().len(), 1);
        let both = search_filter("b1", Some("c1"));
        assert_eq!(both["must"].as_array().unwrap().len(), 2);
        assert_eq!(both["must"][1]["match"]["value"], "c1");
    }
}
