use anyhow::Result;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsClaims {
    pub sub: String,        // admin user documentId
    pub user_id: u64,       // admin user id
    pub email: String,
    pub role: String,
    pub exp: usize,         // expiration timestamp (seconds)
    pub iat: usize,         // issued at (seconds)
}

/// Hash a raw password using bcrypt (cost factor 12)
pub fn hash_password(password: &str) -> Result<String> {
    let hash = bcrypt::hash(password, 12)?;
    Ok(hash)
}

/// Verify a raw password against a bcrypt hash (with plaintext fallback for migration)
pub fn verify_password(password: &str, hash: &str) -> bool {
    if hash.is_empty() {
        return false;
    }
    if let Ok(valid) = bcrypt::verify(password, hash) {
        if valid {
            return true;
        }
    }
    // Allow legacy plaintext match during transition
    password == hash
}

/// Generate a signed JWT for an admin user
pub fn create_jwt(
    document_id: &str,
    user_id: u64,
    email: &str,
    role_code: &str,
    secret: &str,
    expires_in_seconds: u64,
) -> Result<String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = now + (expires_in_seconds as usize);
    let claims = CmsClaims {
        sub: document_id.to_string(),
        user_id,
        email: email.to_string(),
        role: role_code.to_string(),
        exp,
        iat: now,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

/// Verify and decode a JWT token string
pub fn verify_jwt(token: &str, secret: &str) -> Result<CmsClaims> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    let token_data = decode::<CmsClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcrypt_hashing_and_verification() {
        let raw_pass = "SecureP@ssw0rd2026!";
        let hash = hash_password(raw_pass).expect("Hashing should succeed");
        assert!(hash.starts_with("$2"), "Hash must be valid bcrypt format");
        assert!(verify_password(raw_pass, &hash), "Valid password must verify against hash");
        assert!(!verify_password("WrongPassword!", &hash), "Wrong password must be rejected");
        assert!(!verify_password(raw_pass, ""), "Empty hash must be rejected");
    }

    #[test]
    fn test_jwt_issuance_and_verification() {
        let secret = "super-test-secret-key-1234567890123456";
        let doc_id = "doc-uuid-123";
        let token = create_jwt(doc_id, 42, "admin@pixelstack.io", "strapi-super-admin", secret, 3600)
            .expect("JWT creation should succeed");

        let claims = verify_jwt(&token, secret).expect("JWT verification should succeed");
        assert_eq!(claims.sub, doc_id);
        assert_eq!(claims.user_id, 42);
        assert_eq!(claims.email, "admin@pixelstack.io");
        assert_eq!(claims.role, "strapi-super-admin");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_jwt_invalid_secret_rejection() {
        let secret1 = "secret-key-one-12345678901234567890";
        let secret2 = "secret-key-two-12345678901234567890";
        let token = create_jwt("doc-1", 1, "test@test.com", "admin", secret1, 3600).unwrap();

        let result = verify_jwt(&token, secret2);
        assert!(result.is_err(), "JWT verification with wrong secret must fail");
    }

    #[test]
    fn test_jwt_expired_token_rejection() {
        let secret = "secret-key-expired-test-1234567890";
        // Create token with 0 seconds lifespan
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = CmsClaims {
            sub: "doc-expired".to_string(),
            user_id: 99,
            email: "expired@test.com".to_string(),
            role: "editor".to_string(),
            exp: now - 100, // Expired 100s ago
            iat: now - 200,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        ).unwrap();

        let result = verify_jwt(&token, secret);
        assert!(result.is_err(), "Expired JWT must be rejected");
    }
}
