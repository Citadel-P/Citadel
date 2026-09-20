use base64::Engine;

use base64::engine::general_purpose::STANDARD;

use chrono::{DateTime, Utc};

use citadel_identity::{IdentityError, RecoveryCodeService, TotpService, TotpSetup};

use getrandom::fill;

use hmac::{Hmac, KeyInit, Mac};

use sha1::Sha1;

use sha2::Sha256;

const TOTP_SECRET_BYTES: usize = 20;

const TOTP_PERIOD_SECONDS: i64 = 30;

const TOTP_DIGITS_MODULUS: u32 = 1_000_000;

const RECOVERY_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

const RECOVERY_PURPOSE: &[u8] = b"citadel:mfa:recovery-code:v1";

#[derive(Debug, Default)]
pub struct Sha1TotpService;

impl TotpService for Sha1TotpService {
    fn create_setup(
        &self,
        issuer: &str,
        account_name: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<TotpSetup, IdentityError> {
        let mut secret = [0_u8; TOTP_SECRET_BYTES];
        fill(&mut secret).map_err(|_| IdentityError::Credential)?;
        let secret = base32_encode(&secret);
        Ok(TotpSetup {
            otp_auth_uri: self.otp_auth_uri(issuer, account_name, &secret),
            secret,
            expires_at,
        })
    }

    fn verify(&self, secret: &str, code: &str, now: DateTime<Utc>) -> Option<i64> {
        if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let expected = code.parse::<u32>().ok()?;
        let secret = base32_decode(secret)?;
        let current_step = now.timestamp().div_euclid(TOTP_PERIOD_SECONDS);
        [current_step, current_step - 1, current_step + 1]
            .into_iter()
            .find(|step| totp_code(&secret, *step) == expected)
    }

    fn otp_auth_uri(&self, issuer: &str, account_name: &str, secret: &str) -> String {
        let issuer = urlencoding::encode(issuer);
        let account_name = urlencoding::encode(account_name);
        format!(
            "otpauth://totp/{issuer}:{account_name}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits=6&period=30"
        )
    }
}

#[derive(Clone)]
pub struct HmacRecoveryCodeService {
    purpose_key: [u8; 32],
}

impl HmacRecoveryCodeService {
    pub fn new(encryption_key: &[u8]) -> Result<Self, IdentityError> {
        let mut hmac = Hmac::<Sha256>::new_from_slice(encryption_key)
            .map_err(|_| IdentityError::Credential)?;
        hmac.update(RECOVERY_PURPOSE);
        Ok(Self {
            purpose_key: hmac.finalize().into_bytes().into(),
        })
    }
}

impl RecoveryCodeService for HmacRecoveryCodeService {
    fn generate(&self, count: usize) -> Result<Vec<String>, IdentityError> {
        let mut codes = Vec::with_capacity(count);
        for _ in 0..count {
            let mut random = [0_u8; 12];
            fill(&mut random).map_err(|_| IdentityError::Credential)?;
            let value = random
                .map(|byte| RECOVERY_ALPHABET[usize::from(byte) % RECOVERY_ALPHABET.len()] as char);
            codes.push(format!(
                "{}{}{}{}-{}{}{}{}-{}{}{}{}",
                value[0],
                value[1],
                value[2],
                value[3],
                value[4],
                value[5],
                value[6],
                value[7],
                value[8],
                value[9],
                value[10],
                value[11]
            ));
        }
        Ok(codes)
    }

    fn normalize(&self, code: &str) -> String {
        code.trim()
            .chars()
            .filter(|character| *character != '-' && !character.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect()
    }

    fn hash(&self, normalized_code: &str) -> String {
        let mut hmac = Hmac::<Sha256>::new_from_slice(&self.purpose_key)
            .expect("HMAC accepts a fixed SHA-256 key");
        hmac.update(normalized_code.as_bytes());
        STANDARD.encode(hmac.finalize().into_bytes())
    }

    fn verify(&self, normalized_code: &str, expected_hash: &str) -> bool {
        let Ok(expected) = STANDARD.decode(expected_hash) else {
            return false;
        };
        let mut hmac = Hmac::<Sha256>::new_from_slice(&self.purpose_key)
            .expect("HMAC accepts a fixed SHA-256 key");
        hmac.update(normalized_code.as_bytes());
        hmac.verify_slice(&expected).is_ok()
    }
}

fn totp_code(secret: &[u8], step: i64) -> u32 {
    let mut hmac = Hmac::<Sha1>::new_from_slice(secret).expect("HMAC accepts TOTP secrets");
    hmac.update(&(step as u64).to_be_bytes());
    let digest = hmac.finalize().into_bytes();
    let offset = usize::from(digest[digest.len() - 1] & 0x0f);
    let binary = (u32::from(digest[offset]) & 0x7f) << 24
        | u32::from(digest[offset + 1]) << 16
        | u32::from(digest[offset + 2]) << 8
        | u32::from(digest[offset + 3]);
    binary % TOTP_DIGITS_MODULUS
}

fn base32_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        output.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    output
}

fn base32_decode(value: &str) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(value.len() * 5 / 8);
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    for byte in value.bytes() {
        let value = match byte.to_ascii_uppercase() {
            b'A'..=b'Z' => byte.to_ascii_uppercase() - b'A',
            b'2'..=b'7' => byte - b'2' + 26,
            _ => return None,
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn totp_generation_uri_verification_and_adjacent_window_are_compatible() {
        let service = Sha1TotpService;
        let secret = base32_encode(b"12345678901234567890");
        let now = Utc.timestamp_opt(59, 0).unwrap();
        let code = format!("{:06}", totp_code(b"12345678901234567890", 1));

        assert_eq!(service.verify(&secret, &code, now), Some(1));
        assert_eq!(
            service.verify(&secret, &code, now + chrono::Duration::seconds(30)),
            Some(1)
        );
        assert!(service.verify(&secret, "00000x", now).is_none());
        assert_eq!(
            service.otp_auth_uri("Citadel", "owner@example.test", &secret),
            format!(
                "otpauth://totp/Citadel:owner%40example.test?secret={secret}&issuer=Citadel&algorithm=SHA1&digits=6&period=30"
            )
        );
    }

    #[test]
    fn recovery_codes_are_bounded_normalized_and_constant_time_verified() {
        let service = HmacRecoveryCodeService::new(&[9_u8; 32]).unwrap();
        let codes = service.generate(10).unwrap();

        assert_eq!(codes.len(), 10);
        assert!(codes.iter().all(|code| code.len() == 14));
        let normalized = service.normalize(" abcd-efgh ijkl ");
        assert_eq!(normalized, "ABCDEFGHIJKL");
        let hash = service.hash(&normalized);
        assert!(service.verify(&normalized, &hash));
        assert!(!service.verify("ABCDEFGHIJKA", &hash));
    }

    #[test]
    fn base32_round_trips_the_required_twenty_byte_secret() {
        let source = [42_u8; TOTP_SECRET_BYTES];
        assert_eq!(base32_decode(&base32_encode(&source)).unwrap(), source);
    }
}
