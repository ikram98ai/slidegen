use crate::db::Database;
use crate::models::JobRecord;
use anyhow::{Context, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use serde_dynamo::{from_item, from_items, to_item};

impl Database {
    pub async fn get_job(&self, id: &str) -> Result<Option<JobRecord>> {
        let result = self
            .client
            .get_item()
            .table_name("slidegen_jobs")
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await?;

        if let Some(item) = result.item {
            let job: JobRecord = from_item(item)?;
            return Ok(Some(job));
        }

        Ok(None)
    }

    pub async fn save_job(&self, job: &JobRecord) -> Result<()> {
        let item = to_item(job.clone()).context("Failed to serialize job")?;

        self.client
            .put_item()
            .table_name("slidegen_jobs")
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    pub async fn list_jobs_by_subject(&self, subject_id: &str) -> Result<Vec<JobRecord>> {
        let result = self
            .client
            .query()
            .table_name("slidegen_jobs")
            .index_name("subject-index")
            .key_condition_expression("subject_id = :sid")
            .expression_attribute_values(":sid", AttributeValue::S(subject_id.to_string()))
            .send()
            .await?;

        if let Some(items) = result.items {
            let mut jobs: Vec<JobRecord> = from_items(items)?;
            jobs.sort_by_key(|j| std::cmp::Reverse(j.created_at));
            return Ok(jobs);
        }

        Ok(vec![])
    }
}
