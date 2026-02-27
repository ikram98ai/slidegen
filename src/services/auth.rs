use crate::config::Settings;
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::Error as JwtError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub exp: usize,
    pub r#type: String, // "access" or "refresh"
}

pub fn get_password_hash(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

pub fn verify_password(plain_password: &str, hashed_password: &str) -> bool {
    verify(plain_password, hashed_password).unwrap_or(false)
}

pub fn create_access_token(user_id: &str, settings: &Settings) -> Result<String, JwtError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(settings.access_token_expire_days))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        user_id: user_id.to_string(),
        exp: expiration as usize,
        r#type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.secret_key.as_ref()),
    )
}

pub fn create_refresh_token(user_id: &str, settings: &Settings) -> Result<String, JwtError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(settings.refresh_token_expire_days))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        user_id: user_id.to_string(),
        exp: expiration as usize,
        r#type: "refresh".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.secret_key.as_ref()),
    )
}

pub fn verify_token(token: &str, settings: &Settings) -> Result<Claims, JwtError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(settings.secret_key.as_ref()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}
