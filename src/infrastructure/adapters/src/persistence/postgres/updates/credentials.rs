//! Target-owned decryption checks using bounded, typed PostgreSQL reads.
use crate::security::identity::crypto::AesGcmSecretProtector;
use citadel_updates::UpdateError;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn verify_credentials(
    tx: &mut Transaction<'_, Postgres>,
    encryption_key: &[u8],
) -> Result<(), UpdateError> {
    // Bound memory even for maximum-sized internal secrets. No Docker/JSON
    // transport is involved: the target owns this database connection.
    const PAGE_SIZE: i64 = 16;
    let protector = AesGcmSecretProtector::new(encryption_key).map_err(|_| failed())?;
    let mut cursor = (0_i32, Uuid::nil());
    loop {
        let page: Vec<(i32, Uuid, Option<String>)> =
            sqlx::query_as(include_str!("credentials.sql"))
                .bind(cursor.0)
                .bind(cursor.1)
                .bind(PAGE_SIZE)
                .fetch_all(&mut **tx)
                .await
                .map_err(|_| failed())?;
        for (kind, id, envelope) in &page {
            let envelope = envelope.as_deref().ok_or_else(failed)?;
            // The existing protector zeroizes plaintext when it is dropped.
            drop(protector.unprotect(envelope).map_err(|_| failed())?);
            cursor = (*kind, *id);
        }
        if page.len() < PAGE_SIZE as usize {
            return Ok(());
        }
    }
}

fn failed() -> UpdateError {
    UpdateError::Unavailable(
        "Stored credentials could not be decrypted. Maintenance remains active.".into(),
    )
}
