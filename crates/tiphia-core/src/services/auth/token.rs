use crate::{
    config::AuthConfig,
    entities::users,
    error::AppResult,
    services::auth::{Claims, TokenResponse},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};

pub fn decode_token(config: &AuthConfig, token: &str) -> AppResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}

pub fn issue_token(config: &AuthConfig, user: users::Model) -> AppResult<TokenResponse> {
    let now = Utc::now();
    let expires_at = now + Duration::seconds(config.token_ttl_seconds);
    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        role: user.role.clone(),
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;

    Ok(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_at: expires_at.timestamp(),
        user: user.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::AuthConfig, entities::users::UserRole};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    #[test]
    fn token_round_trips() {
        let config = AuthConfig {
            jwt_secret: "test-secret-with-enough-entropy".to_owned(),
            token_ttl_seconds: 60,
        };
        let now = Utc::now();
        let token = encode(
            &Header::new(Algorithm::HS256),
            &Claims {
                sub: 42,
                username: "alice".to_owned(),
                role: UserRole::Admin,
                iat: now.timestamp() as usize,
                exp: (now + Duration::seconds(60)).timestamp() as usize,
            },
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )
        .expect("token");

        assert_eq!(decode_token(&config, &token).expect("claims").sub, 42);
    }
}
