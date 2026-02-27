use crate::config::Settings;
use anyhow::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::Client;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct StorageService {
    client: Client,
    bucket_name: String,
    region: String,
}

impl StorageService {
    pub async fn new(settings: &Settings) -> Self {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(settings.aws_region.clone()))
            .load()
            .await;

        // Similarly to dynamo db, it loads from implicitly available credentials.
        let client = Client::new(&sdk_config);

        StorageService {
            client,
            bucket_name: settings.s3_bucket_name.clone(),
            region: settings.aws_region.clone(),
        }
    }

    pub async fn upload_file(&self, key: &str, content: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(content.into())
            .send()
            .await?;
        Ok(())
    }

    pub async fn download_file(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        let data = resp.body.collect().await?;
        Ok(data.into_bytes().to_vec())
    }

    pub async fn get_presigned_url(&self, key: &str, expiration_secs: u64) -> Result<String> {
        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .presigned(aws_sdk_s3::presigning::PresigningConfig::expires_in(
                Duration::from_secs(expiration_secs),
            )?)
            .await?;

        Ok(presigned_request.uri().to_string())
    }

    pub fn get_public_url(&self, key: &str) -> String {
        format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            self.bucket_name, self.region, key
        )
    }

    pub async fn delete_file(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        Ok(())
    }
}
