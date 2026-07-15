//! Integration tests for the auth service: password hashing and JWT lifecycle.
//!
//! Run with: `make test-int` or `cargo test --test auth_service_test`

use chrono::Utc;
use slidegen::config::Settings;
use slidegen::services::auth::{
    create_access_token, create_refresh_token, get_password_hash, verify_password, verify_token,
};

fn test_settings() -> Settings {
    Settings {
        aws_access_key_id: None,
        aws_secret_access_key: None,
        aws_region: "us-east-1".to_string(),
        s3_bucket_name: "test-bucket".to_string(),
        jobs_queue_url: None,
        debug: true,
        secret_key: "unit-test-secret-key".to_string(),
        algorithm: "HS256".to_string(),
        access_token_expire_days: 7,
        refresh_token_expire_days: 30,
        gemini_api_key: None,
        text_model: "test-model".to_string(),
        embedding_model: "test-embedding".to_string(),
    }
}

#[test]
fn password_hash_roundtrip() {
    let hash = get_password_hash("s3cret-password").unwrap();
    assert_ne!(hash, "s3cret-password");
    assert!(verify_password("s3cret-password", &hash));
    assert!(!verify_password("wrong-password", &hash));
}

#[test]
fn verify_password_handles_invalid_hash_gracefully() {
    assert!(!verify_password("anything", "not-a-bcrypt-hash"));
}

#[test]
fn access_token_roundtrip() {
    let settings = test_settings();
    let token = create_access_token("user-123", &settings).unwrap();
    let claims = verify_token(&token, &settings).unwrap();

    assert_eq!(claims.user_id, "user-123");
    assert_eq!(claims.r#type, "access");
    assert!(claims.exp > Utc::now().timestamp() as usize);
}

#[test]
fn refresh_token_has_refresh_type() {
    let settings = test_settings();
    let token = create_refresh_token("user-123", &settings).unwrap();
    let claims = verify_token(&token, &settings).unwrap();

    assert_eq!(claims.user_id, "user-123");
    assert_eq!(claims.r#type, "refresh");
}

#[test]
fn expired_token_is_rejected() {
    let mut settings = test_settings();
    settings.access_token_expire_days = -1;
    let token = create_access_token("user-123", &settings).unwrap();

    assert!(verify_token(&token, &settings).is_err());
}

#[test]
fn token_signed_with_other_secret_is_rejected() {
    let settings = test_settings();
    let token = create_access_token("user-123", &settings).unwrap();

    let mut other = test_settings();
    other.secret_key = "a-different-secret".to_string();
    assert!(verify_token(&token, &other).is_err());
}

#[test]
fn tampered_token_is_rejected() {
    let settings = test_settings();
    let token = create_access_token("user-123", &settings).unwrap();
    let other_token = create_access_token("attacker", &settings).unwrap();

    // Splice the payload of another token onto this token's signature.
    let parts: Vec<&str> = token.split('.').collect();
    let other_parts: Vec<&str> = other_token.split('.').collect();
    let forged = format!("{}.{}.{}", parts[0], other_parts[1], parts[2]);

    assert!(verify_token(&forged, &settings).is_err());
}

#[test]
fn garbage_token_is_rejected() {
    let settings = test_settings();
    assert!(verify_token("not.a.jwt", &settings).is_err());
    assert!(verify_token("", &settings).is_err());
}
