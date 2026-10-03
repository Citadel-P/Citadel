use chrono::{DateTime, Utc};

use citadel_identity::{
    ChallengeCompletionCommit, DisableMfaCommit, IdentityError, MAXIMUM_SESSIONS_PER_USER,
    MandatoryEnrollmentCommit, MfaCredentialAcceptance, MfaStore, ProfileEnrollmentCommit,
    RegenerateRecoveryCodesCommit, ResetMfaCommit,
};

use citadel_identity::{MfaChallenge, MfaSetupSession, UserMfaRecoveryCode, UserMfaSettings};

use futures_util::future::BoxFuture;

use sqlx::{PgPool, Postgres, Row, Transaction};

use subtle::ConstantTimeEq;

use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;

use crate::persistence::postgres::identity::authentication::store::insert_session;

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
        activity: citadel_activities::ActivityEvent,
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

fn storage(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Storage(error.to_string())
}
