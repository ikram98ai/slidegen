//! Outbound EventBridge notifications. Sibling apps subscribe here instead
//! of sharing the work queue.
//!
//! Detail types: `book.processed`, `chapter.ready`, `job.failed`.
//! Source is always `slidegen`.

use crate::config::Settings;
use anyhow::{Context, Result};
use aws_sdk_eventbridge::types::PutEventsRequestEntry;
use serde::Serialize;
use std::sync::Arc;

const SOURCE: &str = "slidegen";

#[derive(Clone, Debug)]
pub struct EventBus {
    client: Arc<aws_sdk_eventbridge::Client>,
    bus_name: String,
}

impl EventBus {
    pub async fn new(settings: &Settings, bus_name: String) -> Self {
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(settings.aws_region.clone()))
            .load()
            .await;
        Self {
            client: Arc::new(aws_sdk_eventbridge::Client::new(&sdk_config)),
            bus_name,
        }
    }

    pub async fn from_settings(settings: &Settings) -> Option<Self> {
        let name = settings.event_bus_name.clone()?;
        Some(Self::new(settings, name).await)
    }

    pub async fn publish<T: Serialize>(&self, detail_type: &str, detail: &T) -> Result<()> {
        let body = serde_json::to_string(detail).context("Failed to serialize event detail")?;
        let entry = PutEventsRequestEntry::builder()
            .event_bus_name(&self.bus_name)
            .source(SOURCE)
            .detail_type(detail_type)
            .detail(body)
            .build();
        let resp = self
            .client
            .put_events()
            .entries(entry)
            .send()
            .await
            .context("Failed to put EventBridge event")?;
        if resp.failed_entry_count() > 0 {
            anyhow::bail!(
                "EventBridge rejected {} event(s) on bus {}",
                resp.failed_entry_count(),
                self.bus_name
            );
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct BookProcessed {
    pub book_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub chapters_total: i32,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChapterReady {
    pub book_id: String,
    pub chapter_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub package_key: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct JobFailed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub kind: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_payloads_are_stable() {
        let body = serde_json::to_value(&BookProcessed {
            book_id: "b1".into(),
            user_id: "tenant:khan".into(),
            tenant_id: Some("khan".into()),
            job_id: Some("j1".into()),
            chapters_total: 4,
        })
        .unwrap();
        assert_eq!(body["book_id"], "b1");
        assert_eq!(body["chapters_total"], 4);
        assert_eq!(body["tenant_id"], "khan");
    }
}
