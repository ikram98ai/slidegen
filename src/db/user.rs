use crate::db::Database;
use crate::models::User;
use anyhow::{Context, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use serde_dynamo::from_item;
use serde_dynamo::from_items;
use serde_dynamo::to_item;

impl Database {
    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
        let result = self
            .client
            .get_item()
            .table_name("slidegen_users")
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await?;

        if let Some(item) = result.item {
            let user: User = from_item(item)?;
            return Ok(Some(user));
        }

        Ok(None)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let result = self
            .client
            .query()
            .table_name("slidegen_users")
            .index_name("email-index")
            .key_condition_expression("email = :email")
            .expression_attribute_values(":email", AttributeValue::S(email.to_string()))
            .send()
            .await?;

        if let Some(items) = result.items {
            let users: Vec<User> = from_items(items)?;
            return Ok(users.into_iter().next());
        }

        Ok(None)
    }

    pub async fn save_user(&self, user: &User) -> Result<()> {
        let item = to_item(user.clone()).context("Failed to serialize user to DynamoDB item")?;

        self.client
            .put_item()
            .table_name("slidegen_users")
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }
}
