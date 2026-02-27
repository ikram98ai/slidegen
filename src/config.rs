use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    // AWS
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_region: String,
    pub s3_bucket_name: String,

    // Security
    pub debug: bool,
    pub secret_key: String,
    pub algorithm: String,
    pub access_token_expire_days: i64,
    pub refresh_token_expire_days: i64,

    // AI Services
    pub gemini_api_key: Option<String>,
    pub text_model: String,
    pub embedding_model: String,
}

impl Settings {
    pub fn new() -> Self {
        dotenv().ok(); // Ignore if .env not found

        Self {
            aws_access_key_id: env::var("AWS_ACCESS_KEY_ID").ok(),
            aws_secret_access_key: env::var("AWS_SECRET_ACCESS_KEY").ok(),
            aws_region: env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            s3_bucket_name: env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set"),

            debug: env::var("DEBUG")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            secret_key: env::var("SECRET_KEY").expect("SECRET_KEY must be set"),
            algorithm: env::var("ALGORITHM").unwrap_or_else(|_| "HS256".to_string()),
            access_token_expire_days: env::var("ACCESS_TOKEN_EXPIRE_DAYS")
                .unwrap_or_else(|_| "7".to_string())
                .parse()
                .unwrap_or(7),
            refresh_token_expire_days: env::var("REFRESH_TOKEN_EXPIRE_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),

            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            text_model: env::var("TEXT_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string()),
            embedding_model: env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
        }
    }
}
