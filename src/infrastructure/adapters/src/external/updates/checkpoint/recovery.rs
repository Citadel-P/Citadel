//! Operator recovery reuses the system restore format and requires a fresh database.
use super::{integrity::verify_checkpoint_artifacts, storage::cancelled};
use crate::{
    external::{
        backups::system_recovery::{CitadelSystemRestoreOptions, restore_citadel_system},
        updates::{JournalDirectory, RegisteredComposeInstallation},
    },
    persistence::postgres::{
        job_lease::CoreJobLease,
        updates::{finish_checkpoint_restore, require_empty_restore_database},
    },
};
use citadel_instance::environment;
use citadel_updates::{UpdateError, UpdateOperation, UpdatePhase};
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// Run from the exact source build after stopping the original installation and
/// helper. Keep their database, containers and private journal intact. Recovery
/// creates no Docker resources and never changes installer inputs or host state.
pub async fn restore_update_checkpoint(
    checkpoint_directory: &Path,
    options: &CitadelSystemRestoreOptions,
    source_version: &str,
    cancellation: &CancellationToken,
) -> Result<UpdateOperation, UpdateError> {
    cancelled(cancellation)?;
    let stage = JournalDirectory::open(checkpoint_directory.to_owned())?;
    let operation: UpdateOperation = stage.read("operation.json")?;
    let registration: RegisteredComposeInstallation = stage.read("installation.json")?;
    registration.validate()?;
    operation.plan.validate(operation.created_at)?;
    if operation.phase != UpdatePhase::BackingUp
        || operation.revision != 2
        || operation.plan.source.version != source_version
        || operation.plan.source.catalog_sha256
            != citadel_database::MigrationRunner::catalog_sha256().map_err(|_| failed())?
        || operation.instance_id != registration.instance_id
        || operation.plan.registration_id != registration.id
        || stage.path()
            != registration
                .state_directory
                .join(format!("checkpoint-{}", operation.id))
    {
        return Err(failed());
    }
    let receipt =
        verify_checkpoint_artifacts(&stage, &operation, &registration, cancellation).await?;
    if options.bundle != receipt.bundle
        || options
            .secret_encryption_key
            .as_ref()
            .is_none_or(|key| key.len() != 32)
    {
        return Err(failed());
    }
    if !receipt
        .required_external_configuration
        .iter()
        .any(|name| name == environment::CITADEL_SECRETS_ENCRYPTION_KEY)
    {
        use base64::Engine;
        let encoded = super::super::recovery_keys::read_key(
            &receipt.bundle.join("recovery/secret-encryption-key"),
        )?;
        let decoded = zeroize::Zeroizing::new(
            base64::engine::general_purpose::STANDARD
                .decode(std::str::from_utf8(&encoded).map_err(|_| failed())?.trim())
                .map_err(|_| failed())?,
        );
        if Some(decoded.as_slice())
            != options
                .secret_encryption_key
                .as_ref()
                .map(|key| key.as_slice())
        {
            return Err(failed());
        }
    }
    let mut lease = CoreJobLease::acquire(&options.database_url)
        .await
        .map_err(|_| failed())?;
    let pool = crate::persistence::postgres::connection::connect_pool(
        &options.database_url,
        2,
        std::time::Duration::from_secs(5),
    )
    .await
    .map_err(|_| failed())?;
    let token = cancellation.child_token();
    let recovery = async {
        require_empty_restore_database(&pool).await?;
        restore_citadel_system(options, &token)
            .await
            .map_err(|_| failed())?;
        cancelled(&token)?;
        // Re-read immutable evidence before authorizing the restored database.
        if verify_checkpoint_artifacts(&stage, &operation, &registration, &token).await? != receipt
        {
            return Err(failed());
        }
        finish_checkpoint_restore(
            &pool,
            &operation,
            options.secret_encryption_key.as_ref().ok_or_else(failed)?,
            &token,
        )
        .await
    };
    tokio::pin!(recovery);
    let result = tokio::select! {
        result = &mut recovery => result,
        _ = lease.wait_for_loss() => {
            token.cancel();
            let _ = recovery.await;
            Err(failed())
        }
    };
    pool.close().await;
    result
}

fn failed() -> UpdateError {
    UpdateError::Unavailable("Checkpoint recovery was not completed. Preserve the original installation and checkpoint; use the exact source build, original encryption key and a new empty database.".into())
}
