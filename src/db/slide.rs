use crate::db::Database;
use crate::models::Slide;
use anyhow::{Context, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use serde_dynamo::{from_item, from_items, to_item};

impl Database {
    pub async fn get_slide(&self, chapter_id: &str, slide_id: &str) -> Result<Option<Slide>> {
        let result = self
            .client
            .get_item()
            .table_name("slidegen_slides")
            .key("chapter_id", AttributeValue::S(chapter_id.to_string()))
            .key("id", AttributeValue::S(slide_id.to_string()))
            .send()
            .await?;

        if let Some(item) = result.item {
            let slide: Slide = from_item(item)?;
            return Ok(Some(slide));
        }

        Ok(None)
    }

    pub async fn get_slides_by_chapter(&self, chapter_id: &str) -> Result<Vec<Slide>> {
        let result = self
            .client
            .query()
            .table_name("slidegen_slides")
            .key_condition_expression("chapter_id = :cid")
            .expression_attribute_values(":cid", AttributeValue::S(chapter_id.to_string()))
            .send()
            .await?;

        if let Some(items) = result.items {
            let mut slides: Vec<Slide> = from_items(items)?;
            slides.sort_by_key(|s| s.order_index);
            return Ok(slides);
        }

        Ok(vec![])
    }

    pub async fn save_slide(&self, slide: &Slide) -> Result<()> {
        let item = to_item(slide.clone()).context("Failed to serialize slide to DynamoDB item")?;

        self.client
            .put_item()
            .table_name("slidegen_slides")
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    pub async fn delete_slide(&self, chapter_id: &str, slide_id: &str) -> Result<()> {
        self.client
            .delete_item()
            .table_name("slidegen_slides")
            .key("chapter_id", AttributeValue::S(chapter_id.to_string()))
            .key("id", AttributeValue::S(slide_id.to_string()))
            .send()
            .await?;

        Ok(())
    }
}
