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

/// Infers the MIME type from the object key's extension so S3 serves files
/// (especially audio behind presigned URLs) with a playable content type.
fn content_type_for_key(key: &str) -> &'static str {
    let extension = key.rsplit('.').next().unwrap_or("");
    match extension.to_ascii_lowercase().as_str() {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "json" => "application/json",
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        _ => "application/octet-stream",
    }
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

    pub async fn upload_json<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let bytes = serde_json::to_vec(value)?;
        self.upload_file(key, bytes).await
    }

    pub async fn upload_file(&self, key: &str, content: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .content_type(content_type_for_key(key))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_inferred_from_extension() {
        assert_eq!(
            content_type_for_key("subjects/u1/s1.pdf"),
            "application/pdf"
        );
        assert_eq!(content_type_for_key("audio/u1/c1/s1.wav"), "audio/wav");
        assert_eq!(content_type_for_key("audio/u1/c1/s1.mp3"), "audio/mpeg");
        assert_eq!(content_type_for_key("avatars/u1.JPG"), "image/jpeg");
        assert_eq!(content_type_for_key("avatars/u1.png"), "image/png");
        assert_eq!(
            content_type_for_key("extract/pages/0001.json"),
            "application/json"
        );
        assert_eq!(
            content_type_for_key("chapters/c1/index.html"),
            "text/html; charset=utf-8"
        );
    }

    #[test]
    fn content_type_defaults_to_octet_stream() {
        assert_eq!(
            content_type_for_key("no-extension"),
            "application/octet-stream"
        );
        assert_eq!(
            content_type_for_key("weird.xyz"),
            "application/octet-stream"
        );
    }
}
