use crate::db::Database;
use crate::models::Chapter;
use anyhow::{Context, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use serde_dynamo::{from_item, from_items, to_item};

impl Database {
    pub async fn get_chapter(&self, subject_id: &str, chapter_id: &str) -> Result<Option<Chapter>> {
        let result = self
            .client
            .get_item()
            .table_name("slidegen_chapters")
            .key("subject_id", AttributeValue::S(subject_id.to_string()))
            .key("id", AttributeValue::S(chapter_id.to_string()))
            .send()
            .await?;

        if let Some(item) = result.item {
            let chapter: Chapter = from_item(item)?;
            return Ok(Some(chapter));
        }

        Ok(None)
    }

    pub async fn get_chapters_by_subject(&self, subject_id: &str) -> Result<Vec<Chapter>> {
        let result = self
            .client
            .query()
            .table_name("slidegen_chapters")
            .key_condition_expression("subject_id = :sid")
            .expression_attribute_values(":sid", AttributeValue::S(subject_id.to_string()))
            .send()
            .await?;

        if let Some(items) = result.items {
            let mut chapters: Vec<Chapter> = from_items(items)?;
            chapters.sort_by_key(|c| c.order_index);
            return Ok(chapters);
        }

        Ok(vec![])
    }

    pub async fn save_chapter(&self, chapter: &Chapter) -> Result<()> {
        let item =
            to_item(chapter.clone()).context("Failed to serialize chapter to DynamoDB item")?;

        self.client
            .put_item()
            .table_name("slidegen_chapters")
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    pub async fn save_chapters(&self, chapters: Vec<Chapter>) -> Result<()> {
        // Implement batch write
        // Since batch_write_item has limits, inserting sequentially for simplicity,
        // or loop in chunks of 25 items
        for chapter in chapters {
            self.save_chapter(&chapter).await?;
        }
        Ok(())
    }

    pub async fn delete_chapter(&self, subject_id: &str, chapter_id: &str) -> Result<()> {
        self.client
            .delete_item()
            .table_name("slidegen_chapters")
            .key("subject_id", AttributeValue::S(subject_id.to_string()))
            .key("id", AttributeValue::S(chapter_id.to_string()))
            .send()
            .await?;

        Ok(())
    }
}
