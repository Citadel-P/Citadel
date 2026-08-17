use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use citadel_identity::{
    ChallengeCompletionCommit, DisableMfaCommit, IdentityError, MAXIMUM_SESSIONS_PER_USER,
    MandatoryEnrollmentCommit, MfaCredentialAcceptance, MfaStore, ProfileEnrollmentCommit,
    RecoveryCodeService, RegenerateRecoveryCodesCommit, ResetMfaCommit, TotpService, TotpSetup,
};
use citadel_identity::{MfaChallenge, MfaSetupSession, UserMfaRecoveryCode, UserMfaSettings};
use futures_util::future::BoxFuture;
use getrandom::fill;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use sha2::Sha256;
use sqlx::{PgPool, Postgres, Row, Transaction};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::activity_store::insert_activity;
use crate::identity_store::insert_session;

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

#[derive(Clone)]
pub struct PostgresMfaStore {
    pool: PgPool,
}

impl PostgresMfaStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl MfaStore for PostgresMfaStore {
    fn settings(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserMfaSettings>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                "SELECT userid, protectedtotpsecret, lastacceptedtimestep, enabledat, createdat FROM usermfasettings WHERE userid = $1",
            )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(map_settings)
            .transpose()
        })
    }

    fn count_unused_recovery_codes(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<i32, IdentityError>> {
        Box::pin(async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM usermfarecoverycodes WHERE userid = $1 AND usedat IS NULL",
            )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)
            .and_then(|count| i32::try_from(count).map_err(storage))
        })
    }

    fn start_setup(
        &self,
        session: MfaSetupSession,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_user(&mut transaction, session.user_id).await?;
            sqlx::query("DELETE FROM mfasetupsessions WHERE expiresat <= $1")
                .bind(now)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query("DELETE FROM mfasetupsessions WHERE userid = $1")
                .bind(session.user_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            insert_setup_session(&mut transaction, &session).await?;
            transaction.commit().await.map_err(storage)
        })
    }

    fn start_challenge(
        &self,
        challenge: MfaChallenge,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_user(&mut transaction, challenge.user_id).await?;
            sqlx::query("DELETE FROM mfachallenges WHERE expiresat <= $1")
                .bind(now)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query("DELETE FROM mfachallenges WHERE userid = $1")
                .bind(challenge.user_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query(
                "INSERT INTO mfachallenges (id, userid, expiresat, failedattempts, consumedat, createdat) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(challenge.id)
            .bind(challenge.user_id)
            .bind(challenge.expires_at)
            .bind(challenge.failed_attempts)
            .bind(challenge.consumed_at)
            .bind(challenge.created_at)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            transaction.commit().await.map_err(storage)
        })
    }

    fn setup_session(
        &self,
        session_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<MfaSetupSession>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                "SELECT id, userid, protectedtotpsecret, expiresat, consumedat, createdat FROM mfasetupsessions WHERE id = $1 AND consumedat IS NULL AND expiresat > $2",
            )
            .bind(session_id)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(map_setup_session)
            .transpose()
        })
    }

    fn active_setup_session(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<MfaSetupSession>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                "SELECT id, userid, protectedtotpsecret, expiresat, consumedat, createdat FROM mfasetupsessions WHERE userid = $1 AND consumedat IS NULL AND expiresat > $2 ORDER BY createdat DESC LIMIT 1",
            )
            .bind(user_id)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(map_setup_session)
            .transpose()
        })
    }

    fn challenge(
        &self,
        challenge_id: Uuid,
        now: DateTime<Utc>,
        maximum_failed_attempts: i32,
    ) -> BoxFuture<'_, Result<Option<MfaChallenge>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                "SELECT id, userid, expiresat, failedattempts, consumedat, createdat FROM mfachallenges WHERE id = $1 AND consumedat IS NULL AND expiresat > $2 AND failedattempts < $3",
            )
            .bind(challenge_id)
            .bind(now)
            .bind(maximum_failed_attempts)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(map_challenge)
            .transpose()
        })
    }

    fn confirm_profile_enrollment(
        &self,
        commit: ProfileEnrollmentCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            if !consume_setup_session(
                &mut transaction,
                commit.setup_session_id,
                commit.settings.user_id,
                &commit.settings.protected_totp_secret,
                commit.now,
            )
            .await?
            {
                return Ok(false);
            }
            upsert_settings(&mut transaction, &commit.settings).await?;
            replace_recovery_codes(
                &mut transaction,
                commit.settings.user_id,
                &commit.recovery_codes,
            )
            .await?;
            revoke_other_sessions(
                &mut transaction,
                commit.settings.user_id,
                commit.current_session_id,
            )
            .await?;
            insert_activity(&mut transaction, &commit.activity).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn confirm_mandatory_enrollment(
        &self,
        commit: MandatoryEnrollmentCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            if !consume_setup_session(
                &mut transaction,
                commit.setup_session_id,
                commit.settings.user_id,
                &commit.settings.protected_totp_secret,
                commit.now,
            )
            .await?
            {
                return Ok(false);
            }
            upsert_settings(&mut transaction, &commit.settings).await?;
            replace_recovery_codes(
                &mut transaction,
                commit.settings.user_id,
                &commit.recovery_codes,
            )
            .await?;
            insert_session(
                &mut transaction,
                &commit.prepared_session.session,
                commit.prepared_session.expected_password_hash.as_deref(),
                MAXIMUM_SESSIONS_PER_USER,
            )
            .await?;
            insert_activity(&mut transaction, &commit.activity).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn complete_challenge(
        &self,
        commit: ChallengeCompletionCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let active = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM mfachallenges WHERE id = $1 AND userid = $2 AND consumedat IS NULL AND expiresat > $3 AND failedattempts < $4 FOR UPDATE",
            )
            .bind(commit.challenge_id)
            .bind(commit.user_id)
            .bind(commit.now)
            .bind(commit.maximum_failed_attempts)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?;
            if active.is_none()
                || !accept_credential(
                    &mut transaction,
                    commit.user_id,
                    &commit.acceptance,
                    commit.now,
                )
                .await?
            {
                return Ok(false);
            }
            let consumed = sqlx::query(
                "UPDATE mfachallenges SET consumedat = $3 WHERE id = $1 AND userid = $2 AND consumedat IS NULL AND expiresat > $3 AND failedattempts < $4",
            )
            .bind(commit.challenge_id)
            .bind(commit.user_id)
            .bind(commit.now)
            .bind(commit.maximum_failed_attempts)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if consumed != 1 {
                return Ok(false);
            }
            insert_session(
                &mut transaction,
                &commit.prepared_session.session,
                commit.prepared_session.expected_password_hash.as_deref(),
                MAXIMUM_SESSIONS_PER_USER,
            )
            .await?;
            if let Some(activity) = &commit.activity {
                insert_activity(&mut transaction, activity).await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn record_challenge_failure(
        &self,
        challenge_id: Uuid,
        maximum_failed_attempts: i32,
        now: DateTime<Utc>,
        activity: citadel_domain::ActivityEvent,
    ) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let changed = sqlx::query(
                "UPDATE mfachallenges SET failedattempts = failedattempts + 1 WHERE id = $1 AND consumedat IS NULL AND expiresat > $2 AND failedattempts < $3",
            )
            .bind(challenge_id)
            .bind(now)
            .bind(maximum_failed_attempts)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if changed != 1 {
                return Ok(false);
            }
            insert_activity(&mut transaction, &activity).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn disable(&self, commit: DisableMfaCommit) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            if !accept_credential(
                &mut transaction,
                commit.user_id,
                &commit.acceptance,
                commit.now,
            )
            .await?
            {
                return Ok(false);
            }
            delete_mfa_state(&mut transaction, commit.user_id).await?;
            revoke_other_sessions(&mut transaction, commit.user_id, commit.current_session_id)
                .await?;
            insert_activity(&mut transaction, &commit.activity).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn regenerate_recovery_codes(
        &self,
        commit: RegenerateRecoveryCodesCommit,
    ) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let changed = sqlx::query(
                "UPDATE usermfasettings SET lastacceptedtimestep = $2 WHERE userid = $1 AND protectedtotpsecret = $3 AND (lastacceptedtimestep IS NULL OR lastacceptedtimestep < $2)",
            )
            .bind(commit.user_id)
            .bind(commit.matched_time_step)
            .bind(&commit.expected_protected_secret)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if changed != 1 {
                return Ok(false);
            }
            replace_recovery_codes(&mut transaction, commit.user_id, &commit.recovery_codes)
                .await?;
            insert_activity(&mut transaction, &commit.activity).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn reset(&self, commit: ResetMfaCommit) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let exists =
                sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE id = $1 FOR UPDATE")
                    .bind(commit.user_id)
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(storage)?;
            if exists.is_none() {
                return Err(IdentityError::NotFound);
            }
            delete_mfa_state(&mut transaction, commit.user_id).await?;
            sqlx::query("DELETE FROM refreshtokens WHERE userid = $1")
                .bind(commit.user_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            insert_activity(&mut transaction, &commit.activity).await?;
            transaction.commit().await.map_err(storage)
        })
    }
}

async fn insert_setup_session(
    transaction: &mut Transaction<'_, Postgres>,
    session: &MfaSetupSession,
) -> Result<(), IdentityError> {
    sqlx::query(
        "INSERT INTO mfasetupsessions (id, userid, protectedtotpsecret, expiresat, consumedat, createdat) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session.id)
    .bind(session.user_id)
    .bind(&session.protected_totp_secret)
    .bind(session.expires_at)
    .bind(session.consumed_at)
    .bind(session.created_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn consume_setup_session(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    user_id: Uuid,
    expected_secret: &str,
    now: DateTime<Utc>,
) -> Result<bool, IdentityError> {
    Ok(sqlx::query(
        "UPDATE mfasetupsessions SET consumedat = $4 WHERE id = $1 AND userid = $2 AND protectedtotpsecret = $3 AND consumedat IS NULL AND expiresat > $4",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(expected_secret)
    .bind(now)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?
    .rows_affected()
        == 1)
}

async fn upsert_settings(
    transaction: &mut Transaction<'_, Postgres>,
    settings: &UserMfaSettings,
) -> Result<(), IdentityError> {
    sqlx::query(
        "INSERT INTO usermfasettings (userid, protectedtotpsecret, lastacceptedtimestep, enabledat, createdat) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (userid) DO UPDATE SET protectedtotpsecret = EXCLUDED.protectedtotpsecret, lastacceptedtimestep = EXCLUDED.lastacceptedtimestep, enabledat = EXCLUDED.enabledat",
    )
    .bind(settings.user_id)
    .bind(&settings.protected_totp_secret)
    .bind(settings.last_accepted_time_step)
    .bind(settings.enabled_at)
    .bind(settings.created_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn replace_recovery_codes(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    recovery_codes: &[UserMfaRecoveryCode],
) -> Result<(), IdentityError> {
    sqlx::query("DELETE FROM usermfarecoverycodes WHERE userid = $1")
        .bind(user_id)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    for code in recovery_codes {
        if code.user_id != user_id {
            return Err(IdentityError::Validation(
                "Recovery code belongs to a different User.".to_owned(),
            ));
        }
        sqlx::query(
            "INSERT INTO usermfarecoverycodes (id, userid, codehash, usedat, createdat) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(code.id)
        .bind(code.user_id)
        .bind(&code.code_hash)
        .bind(code.used_at)
        .bind(code.created_at)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    }
    Ok(())
}

async fn accept_credential(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    acceptance: &MfaCredentialAcceptance,
    now: DateTime<Utc>,
) -> Result<bool, IdentityError> {
    let affected = match acceptance {
        MfaCredentialAcceptance::Totp {
            matched_time_step,
            expected_protected_secret,
        } => {
            sqlx::query(
                "UPDATE usermfasettings SET lastacceptedtimestep = $2 WHERE userid = $1 AND protectedtotpsecret = $3 AND (lastacceptedtimestep IS NULL OR lastacceptedtimestep < $2)",
            )
            .bind(user_id)
            .bind(matched_time_step)
            .bind(expected_protected_secret)
            .execute(&mut **transaction)
            .await
            .map_err(storage)?
            .rows_affected()
        }
        MfaCredentialAcceptance::RecoveryCode { code_hash } => {
            let unused_codes = sqlx::query_as::<_, (Uuid, String)>(
                "SELECT id, codehash FROM usermfarecoverycodes WHERE userid = $1 AND usedat IS NULL FOR UPDATE",
            )
            .bind(user_id)
            .fetch_all(&mut **transaction)
            .await
            .map_err(storage)?;
            let mut matched_id = None;
            for (id, stored_hash) in unused_codes {
                if bool::from(stored_hash.as_bytes().ct_eq(code_hash.as_bytes())) {
                    matched_id = Some(id);
                }
            }
            let Some(matched_id) = matched_id else {
                return Ok(false);
            };
            sqlx::query(
                "UPDATE usermfarecoverycodes SET usedat = $2 WHERE id = $1 AND usedat IS NULL",
            )
            .bind(matched_id)
            .bind(now)
            .execute(&mut **transaction)
            .await
            .map_err(storage)?
            .rows_affected()
        }
    };
    Ok(affected == 1)
}

async fn lock_user(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), IdentityError> {
    let exists = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(IdentityError::InvalidCredentials)
    }
}

async fn delete_mfa_state(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), IdentityError> {
    for table in [
        "usermfarecoverycodes",
        "usermfasettings",
        "mfasetupsessions",
        "mfachallenges",
    ] {
        let query = format!("DELETE FROM {table} WHERE userid = $1");
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(user_id)
            .execute(&mut **transaction)
            .await
            .map_err(storage)?;
    }
    Ok(())
}

async fn revoke_other_sessions(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    current_session_id: Option<Uuid>,
) -> Result<(), IdentityError> {
    sqlx::query("DELETE FROM refreshtokens WHERE userid = $1 AND ($2::uuid IS NULL OR id <> $2)")
        .bind(user_id)
        .bind(current_session_id)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    Ok(())
}

fn map_settings(row: sqlx::postgres::PgRow) -> Result<UserMfaSettings, IdentityError> {
    Ok(UserMfaSettings {
        user_id: row.try_get("userid").map_err(storage)?,
        protected_totp_secret: row.try_get("protectedtotpsecret").map_err(storage)?,
        last_accepted_time_step: row.try_get("lastacceptedtimestep").map_err(storage)?,
        enabled_at: row.try_get("enabledat").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

fn map_setup_session(row: sqlx::postgres::PgRow) -> Result<MfaSetupSession, IdentityError> {
    Ok(MfaSetupSession {
        id: row.try_get("id").map_err(storage)?,
        user_id: row.try_get("userid").map_err(storage)?,
        protected_totp_secret: row.try_get("protectedtotpsecret").map_err(storage)?,
        expires_at: row.try_get("expiresat").map_err(storage)?,
        consumed_at: row.try_get("consumedat").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

fn map_challenge(row: sqlx::postgres::PgRow) -> Result<MfaChallenge, IdentityError> {
    Ok(MfaChallenge {
        id: row.try_get("id").map_err(storage)?,
        user_id: row.try_get("userid").map_err(storage)?,
        expires_at: row.try_get("expiresat").map_err(storage)?,
        failed_attempts: row.try_get("failedattempts").map_err(storage)?,
        consumed_at: row.try_get("consumedat").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
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

fn storage(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Storage(error.to_string())
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
