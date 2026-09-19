use dotenvy::dotenv;
use std::env;

/// Tenant id + shared secret accepted via the `X-Api-Key` header.
pub type ServiceApiKey = (String, String);

#[derive(Debug, Clone)]
pub struct Settings {
    // AWS
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_region: String,
    pub s3_bucket_name: String,
    /// SQS queue for background jobs. When unset, jobs run in-process
    /// (fine locally, unreliable on Lambda).
    pub jobs_queue_url: Option<String>,
    /// EventBridge bus for `book.processed` / `chapter.ready` fan-out.
    pub event_bus_name: Option<String>,
    /// After TOC analysis, enqueue the first chapter and then the next
    /// as each package completes.
    pub auto_generate_chapters: bool,

    // Security
    pub debug: bool,
    pub secret_key: String,
    pub algorithm: String,
    pub access_token_expire_days: i64,
    pub refresh_token_expire_days: i64,
    /// `tenant_id:secret,other:secret` — used by sibling services (khaneducation, …).
    pub service_api_keys: Vec<ServiceApiKey>,

    // AI Services
    pub gemini_api_key: Option<String>,
    pub text_model: String,
    pub embedding_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    /// Loads settings from the environment (and `.env` if present).
    pub fn new() -> Self {
        dotenv().ok(); // Ignore if .env not found

        Self {
            aws_access_key_id: env::var("AWS_ACCESS_KEY_ID").ok(),
            aws_secret_access_key: env::var("AWS_SECRET_ACCESS_KEY").ok(),
            aws_region: env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            s3_bucket_name: env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set"),
            jobs_queue_url: env::var("JOBS_QUEUE_URL").ok().filter(|v| !v.is_empty()),
            event_bus_name: env::var("EVENT_BUS_NAME").ok().filter(|v| !v.is_empty()),
            auto_generate_chapters: env::var("AUTO_GENERATE_CHAPTERS")
                .ok()
                .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no"))
                .unwrap_or(true),

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
            service_api_keys: parse_service_api_keys(
                &env::var("SERVICE_API_KEYS").unwrap_or_default(),
            ),

            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            text_model: env::var("TEXT_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string()),
            embedding_model: env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "gemini-embedding-001".to_string()),
        }
    }

    pub fn tenant_for_api_key(&self, key: &str) -> Option<&str> {
        self.service_api_keys
            .iter()
            .find_map(|(tenant, secret)| constant_eq(secret, key).then_some(tenant.as_str()))
    }

    pub fn for_tests() -> Self {
        Self {
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_region: "us-east-1".into(),
            s3_bucket_name: "test-bucket".into(),
            jobs_queue_url: None,
            event_bus_name: None,
            auto_generate_chapters: true,
            debug: true,
            secret_key: "unit-test-secret-key".into(),
            algorithm: "HS256".into(),
            access_token_expire_days: 7,
            refresh_token_expire_days: 30,
            service_api_keys: vec![],
            gemini_api_key: None,
            text_model: "test-model".into(),
            embedding_model: "test-embedding".into(),
        }
    }
}

/// `khaneducation:sk_xxx,knoio:sk_yyy`
pub fn parse_service_api_keys(raw: &str) -> Vec<ServiceApiKey> {
    raw.split(',')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            let (id, key) = pair.split_once(':')?;
            let id = id.trim();
            let key = key.trim();
            if id.is_empty() || key.is_empty() {
                None
            } else {
                Some((id.to_string(), key.to_string()))
            }
        })
        .collect()
}

fn constant_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_service_api_keys_skips_junk() {
        let keys = parse_service_api_keys(" khaneducation:sk_live_1 , ,knoio:sk_2,bad");
        assert_eq!(
            keys,
            vec![
                ("khaneducation".into(), "sk_live_1".into()),
                ("knoio".into(), "sk_2".into()),
            ]
        );
    }

    #[test]
    fn tenant_lookup_is_length_safe() {
        let mut settings = Settings::for_tests();
        settings.service_api_keys = vec![("khan".into(), "sk_abc".into())];
        assert_eq!(settings.tenant_for_api_key("sk_abc"), Some("khan"));
        assert_eq!(settings.tenant_for_api_key("sk_ab"), None);
        assert_eq!(settings.tenant_for_api_key("sk_xyz"), None);
    }

    #[test]
    fn constant_eq_rejects_prefix() {
        assert!(constant_eq("abcd", "abcd"));
        assert!(!constant_eq("abcd", "abc"));
        assert!(!constant_eq("abcd", "abce"));
    }
}
