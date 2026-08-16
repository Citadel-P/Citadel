use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::password_hash::{PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use citadel_application::{
    AccessTokenClaims, IdentityError, PasswordHasher, RefreshTokenClaims, ServiceAccountTokenCodec,
    SessionTokenCodec, token_digest,
};
use citadel_domain::{ActorId, AuthenticatedPrincipalType};
use getrandom::fill;
use jsonwebtoken::{Algorithm as JwtAlgorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const PASSWORD_PREFIX: &str = "cit_pwd_v1$";
const SECRET_PREFIX: &str = "cit_secret_v1.";
const SESSION_VERSION: &str = "cit_session_v1";
const SERVICE_ACCOUNT_PREFIX: &str = "cit_sa_";
const SERVICE_ACCOUNT_SECRET_BYTES: usize = 32;

pub struct Argon2PasswordHasher {
    argon2: Argon2<'static>,
    dummy_hash: String,
}

impl Default for Argon2PasswordHasher {
    fn default() -> Self {
        let params = Params::new(19_456, 2, 1, None).expect("fixed Argon2 parameters are valid");
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let salt =
            SaltString::encode_b64(&[0x5a_u8; 16]).expect("fixed dummy password salt is valid");
        let dummy_hash = argon2
            .hash_password(b"citadel-invalid-credential", &salt)
            .expect("fixed dummy password can be hashed");
        Self {
            argon2,
            dummy_hash: format!("{PASSWORD_PREFIX}{dummy_hash}"),
        }
    }
}

impl PasswordHasher for Argon2PasswordHasher {
    fn hash(&self, password: &str) -> Result<String, IdentityError> {
        let mut salt_bytes = [0_u8; 16];
        fill(&mut salt_bytes).map_err(|_| IdentityError::Credential)?;
        let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| IdentityError::Credential)?;
        salt_bytes.zeroize();
        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| IdentityError::Credential)?;
        Ok(format!("{PASSWORD_PREFIX}{hash}"))
    }

    fn verify(&self, password: &str, encoded_hash: &str) -> bool {
        let Some(encoded_hash) = encoded_hash.strip_prefix(PASSWORD_PREFIX) else {
            return false;
        };
        PasswordHash::new(encoded_hash).is_ok_and(|hash| {
            self.argon2
                .verify_password(password.as_bytes(), &hash)
                .is_ok()
        })
    }

    fn dummy_hash(&self) -> String {
        self.dummy_hash.clone()
    }
}

#[derive(Debug, Clone)]
pub struct JwtSessionTokenCodec {
    encoding: EncodingKey,
    decoding: DecodingKey,
    issuer: String,
    audience: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    ver: String,
    token_type: String,
    sub: String,
    actor_id: Option<Uuid>,
    principal_type: Option<AuthenticatedPrincipalType>,
    session_id: Option<Uuid>,
    iss: String,
    aud: String,
    iat: i64,
    nbf: i64,
    exp: i64,
    jti: Uuid,
}

impl JwtSessionTokenCodec {
    pub fn new(key: &[u8], issuer: String, audience: String) -> Result<Self, IdentityError> {
        if key.len() < 32 || issuer.trim().is_empty() || audience.trim().is_empty() {
            return Err(IdentityError::Credential);
        }
        Ok(Self {
            encoding: EncodingKey::from_secret(key),
            decoding: DecodingKey::from_secret(key),
            issuer,
            audience,
        })
    }

    fn encode(&self, claims: &JwtClaims) -> Result<String, IdentityError> {
        jsonwebtoken::encode(&Header::new(JwtAlgorithm::HS256), claims, &self.encoding)
            .map_err(|_| IdentityError::Credential)
    }

    fn decode(&self, token: &str, expected_type: &str) -> Result<JwtClaims, IdentityError> {
        let mut validation = Validation::new(JwtAlgorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.validate_nbf = true;
        validation.leeway = 0;
        let claims = jsonwebtoken::decode::<JwtClaims>(token, &self.decoding, &validation)
            .map_err(|_| IdentityError::Credential)?
            .claims;
        if claims.ver != SESSION_VERSION || claims.token_type != expected_type {
            return Err(IdentityError::Credential);
        }
        Ok(claims)
    }
}

impl SessionTokenCodec for JwtSessionTokenCodec {
    fn encode_access(&self, claims: &AccessTokenClaims) -> Result<String, IdentityError> {
        self.encode(&JwtClaims {
            ver: SESSION_VERSION.to_owned(),
            token_type: "access".to_owned(),
            sub: claims.subject_id.to_string(),
            actor_id: Some(claims.actor_id.value()),
            principal_type: Some(claims.principal_type),
            session_id: None,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: claims.issued_at.timestamp(),
            nbf: claims.issued_at.timestamp(),
            exp: claims.expires_at.timestamp(),
            jti: Uuid::now_v7(),
        })
    }

    fn decode_access(&self, token: &str) -> Result<AccessTokenClaims, IdentityError> {
        let claims = self.decode(token, "access")?;
        Ok(AccessTokenClaims {
            subject_id: Uuid::parse_str(&claims.sub).map_err(|_| IdentityError::Credential)?,
            actor_id: ActorId::new(claims.actor_id.ok_or(IdentityError::Credential)?),
            principal_type: claims.principal_type.ok_or(IdentityError::Credential)?,
            issued_at: timestamp(claims.iat)?,
            expires_at: timestamp(claims.exp)?,
        })
    }

    fn encode_refresh(&self, claims: &RefreshTokenClaims) -> Result<String, IdentityError> {
        self.encode(&JwtClaims {
            ver: SESSION_VERSION.to_owned(),
            token_type: "refresh".to_owned(),
            sub: claims.session_id.to_string(),
            actor_id: None,
            principal_type: None,
            session_id: Some(claims.session_id),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: claims.issued_at.timestamp(),
            nbf: claims.issued_at.timestamp(),
            exp: claims.expires_at.timestamp(),
            jti: Uuid::now_v7(),
        })
    }

    fn decode_refresh(&self, token: &str) -> Result<RefreshTokenClaims, IdentityError> {
        let claims = self.decode(token, "refresh")?;
        let session_id = claims.session_id.ok_or(IdentityError::Credential)?;
        if claims.sub != session_id.to_string() {
            return Err(IdentityError::Credential);
        }
        Ok(RefreshTokenClaims {
            session_id,
            issued_at: timestamp(claims.iat)?,
            expires_at: timestamp(claims.exp)?,
        })
    }
}

fn timestamp(value: i64) -> Result<DateTime<Utc>, IdentityError> {
    DateTime::from_timestamp(value, 0).ok_or(IdentityError::Credential)
}

#[derive(Debug, Default)]
pub struct OpaqueServiceAccountTokenCodec;

impl OpaqueServiceAccountTokenCodec {
    pub fn issue(credential_id: Uuid) -> Result<(String, [u8; 32]), IdentityError> {
        let mut secret = Zeroizing::new([0_u8; SERVICE_ACCOUNT_SECRET_BYTES]);
        fill(secret.as_mut()).map_err(|_| IdentityError::Credential)?;
        let digest = token_digest(secret.as_ref());
        let token = format!(
            "{SERVICE_ACCOUNT_PREFIX}{}.{}",
            credential_id.simple(),
            URL_SAFE_NO_PAD.encode(secret.as_ref())
        );
        Ok((token, digest))
    }
}

impl ServiceAccountTokenCodec for OpaqueServiceAccountTokenCodec {
    fn issue(&self, credential_id: Uuid) -> Result<(String, [u8; 32]), IdentityError> {
        Self::issue(credential_id)
    }

    fn parse_and_hash(&self, token: &str) -> Option<(Uuid, [u8; 32])> {
        const TOKEN_LENGTH: usize = 7 + 32 + 1 + 43;
        if !token.is_ascii()
            || token.len() != TOKEN_LENGTH
            || !token.starts_with(SERVICE_ACCOUNT_PREFIX)
        {
            return None;
        }
        let body = &token[SERVICE_ACCOUNT_PREFIX.len()..];
        let (id, encoded_secret) = body.split_once('.')?;
        if id.len() != 32
            || encoded_secret.len() != 43
            || id
                .bytes()
                .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
            || encoded_secret
                .bytes()
                .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'-' && byte != b'_')
        {
            return None;
        }
        let credential_id = Uuid::parse_str(id).ok()?;
        let mut secret = Zeroizing::new([0_u8; SERVICE_ACCOUNT_SECRET_BYTES]);
        let decoded = URL_SAFE_NO_PAD
            .decode_slice(encoded_secret.as_bytes(), secret.as_mut())
            .ok()?;
        if decoded != SERVICE_ACCOUNT_SECRET_BYTES {
            return None;
        }
        Some((credential_id, token_digest(secret.as_ref())))
    }
}

pub struct AesGcmSecretProtector {
    cipher: Aes256Gcm,
}

impl AesGcmSecretProtector {
    pub fn new(key: &[u8]) -> Result<Self, IdentityError> {
        let key: &[u8; 32] = key.try_into().map_err(|_| IdentityError::Credential)?;
        Ok(Self {
            cipher: Aes256Gcm::new(key.into()),
        })
    }

    pub fn protect(&self, plaintext: &[u8]) -> Result<String, IdentityError> {
        let mut nonce = [0_u8; 12];
        fill(&mut nonce).map_err(|_| IdentityError::Credential)?;
        let nonce_value = Nonce::from(nonce);
        let encrypted = self
            .cipher
            .encrypt(&nonce_value, plaintext)
            .map_err(|_| IdentityError::Credential)?;
        Ok(format!(
            "{SECRET_PREFIX}{}.{}",
            URL_SAFE_NO_PAD.encode(nonce),
            URL_SAFE_NO_PAD.encode(encrypted)
        ))
    }

    pub fn unprotect(&self, envelope: &str) -> Result<Zeroizing<Vec<u8>>, IdentityError> {
        let body = envelope
            .strip_prefix(SECRET_PREFIX)
            .ok_or(IdentityError::Credential)?;
        let (nonce, ciphertext) = body.split_once('.').ok_or(IdentityError::Credential)?;
        let mut nonce_bytes = URL_SAFE_NO_PAD
            .decode(nonce.as_bytes())
            .map_err(|_| IdentityError::Credential)?;
        if nonce_bytes.len() != 12 {
            nonce_bytes.zeroize();
            return Err(IdentityError::Credential);
        }
        let nonce_array: [u8; 12] = nonce_bytes
            .as_slice()
            .try_into()
            .map_err(|_| IdentityError::Credential)?;
        nonce_bytes.zeroize();
        let nonce = Nonce::from(nonce_array);
        let ciphertext = URL_SAFE_NO_PAD
            .decode(ciphertext.as_bytes())
            .map_err(|_| IdentityError::Credential)?;
        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext.as_ref())
            .map_err(|_| IdentityError::Credential)?;
        Ok(Zeroizing::new(plaintext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hashes_are_versioned_and_reject_unknown_formats() {
        let hasher = Argon2PasswordHasher::default();
        let hash = hasher.hash("correct-horse-battery-staple").unwrap();
        assert!(hash.starts_with(PASSWORD_PREFIX));
        assert!(hasher.verify("correct-horse-battery-staple", &hash));
        assert!(!hasher.verify("wrong-password-is-long-enough", &hash));
        assert!(!hasher.verify(
            "correct-horse-battery-staple",
            &hash[PASSWORD_PREFIX.len()..]
        ));
    }

    #[test]
    fn service_account_tokens_are_exact_and_hash_only_the_random_secret() {
        let id = Uuid::now_v7();
        let (token, expected) = OpaqueServiceAccountTokenCodec::issue(id).unwrap();
        let parsed = OpaqueServiceAccountTokenCodec
            .parse_and_hash(&token)
            .unwrap();
        assert_eq!(parsed, (id, expected));
        assert_eq!(token.len(), 83);
        assert!(
            OpaqueServiceAccountTokenCodec
                .parse_and_hash(&format!("{token}="))
                .is_none()
        );
        assert!(
            OpaqueServiceAccountTokenCodec
                .parse_and_hash(&token.to_ascii_uppercase())
                .is_none()
        );
    }

    #[test]
    fn session_tokens_cannot_cross_token_types() {
        let codec =
            JwtSessionTokenCodec::new(&[7_u8; 32], "issuer".into(), "audience".into()).unwrap();
        let now = Utc::now();
        let access = codec
            .encode_access(&AccessTokenClaims {
                subject_id: Uuid::now_v7(),
                actor_id: ActorId::new(Uuid::now_v7()),
                principal_type: AuthenticatedPrincipalType::User,
                issued_at: now,
                expires_at: now + chrono::Duration::minutes(15),
            })
            .unwrap();
        assert!(codec.decode_access(&access).is_ok());
        assert!(codec.decode_refresh(&access).is_err());
    }

    #[test]
    fn secret_envelopes_are_versioned_and_authenticated() {
        let protector = AesGcmSecretProtector::new(&[9_u8; 32]).unwrap();
        let protected = protector.protect(b"sensitive").unwrap();
        assert!(protected.starts_with(SECRET_PREFIX));
        assert_eq!(
            protector.unprotect(&protected).unwrap().as_slice(),
            b"sensitive"
        );
        let body = protected.strip_prefix(SECRET_PREFIX).unwrap();
        let (nonce, ciphertext) = body.split_once('.').unwrap();
        let mut corrupted = URL_SAFE_NO_PAD.decode(ciphertext).unwrap();
        corrupted[0] ^= 1;
        let corrupted = format!(
            "{SECRET_PREFIX}{nonce}.{}",
            URL_SAFE_NO_PAD.encode(corrupted)
        );
        assert!(protector.unprotect(&corrupted).is_err());
    }
}
