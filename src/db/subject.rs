use crate::db::Database;
use crate::models::Subject;
use anyhow::{Context, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use serde_dynamo::{from_item, from_items, to_item};

impl Database {
    pub async fn get_subject(&self, id: &str) -> Result<Option<Subject>> {
        let result = self
            .client
            .get_item()
            .table_name("slidegen_subjects")
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await?;

        if let Some(item) = result.item {
            let subject: Subject = from_item(item)?;
            return Ok(Some(subject));
        }

        Ok(None)
    }

    pub async fn list_subjects(&self, limit: i32) -> Result<Vec<Subject>> {
        let result = self
            .client
            .scan()
            .table_name("slidegen_subjects")
            .limit(limit)
            .send()
            .await?;

        if let Some(items) = result.items {
            let subjects: Vec<Subject> = from_items(items)?;
            return Ok(subjects);
        }

        Ok(vec![])
    }

    pub async fn list_public_subjects(&self, limit: i32) -> Result<Vec<Subject>> {
        let result = self
            .client
            .scan()
            .table_name("slidegen_subjects")
            .filter_expression("is_public = :pub")
            .expression_attribute_values(":pub", AttributeValue::Bool(true))
            .limit(limit)
            .send()
            .await?;

        if let Some(items) = result.items {
            let subjects: Vec<Subject> = from_items(items)?;
            return Ok(subjects);
        }

        Ok(vec![])
    }

    pub async fn get_user_subjects_by_id(
        &self,
        user_id: &str,
        public_only: bool,
    ) -> Result<Vec<Subject>> {
        let query = self
            .client
            .query()
            .table_name("slidegen_subjects")
            .index_name("user-index")
            .key_condition_expression("user_id = :uid")
            .expression_attribute_values(":uid", AttributeValue::S(user_id.to_string()));

        let result = if public_only {
            query
                .filter_expression("is_public = :pub")
                .expression_attribute_values(":pub", AttributeValue::Bool(true))
                .send()
                .await?
        } else {
            query.send().await?
        };

        if let Some(items) = result.items {
            let subjects: Vec<Subject> = from_items(items)?;
            return Ok(subjects);
        }

        Ok(vec![])
    }

    pub async fn save_subject(&self, subject: &Subject) -> Result<()> {
        let item = to_item(subject.clone()).context("Failed to serialize subject")?;

        self.client
            .put_item()
            .table_name("slidegen_subjects")
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    pub async fn delete_subject(&self, id: &str) -> Result<()> {
        self.client
            .delete_item()
            .table_name("slidegen_subjects")
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await?;

        Ok(())
    }
}
