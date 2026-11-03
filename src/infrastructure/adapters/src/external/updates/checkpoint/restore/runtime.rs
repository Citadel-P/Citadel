use super::super::super::{
    InstallationLock, JournalDirectory,
    docker::{LocalDocker, single},
};
use super::{
    evidence::{self, failed},
    sandbox::Sandbox,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateOperation, is_sha256};
use serde::{Deserialize, Serialize};
use std::{
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct CheckpointRestoreBudget {
    pub timeout: Duration,
    /// Bound tmpfs capacity; exhaustion fails verification without touching source storage.
    pub maximum_bytes: u64,
}

/// Successful isolated PostgreSQL restore, authenticated decryption and cleanup, not
/// writer quiescence, replacement authorization or source resume permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoredUpdateCheckpoint {
    pub format: u32,
    pub operation_id: Uuid,
    pub fence: Uuid,
    pub instance_id: Uuid,
    pub bundle_checksums_sha256: String,
    pub catalog_sha256: String,
    pub database_image: String,
    pub sandbox_id: String,
    /// Zero means there were no stored encrypted values to verify.
    pub decrypted_values: u64,
}

/// Helper-owned work under the installation lock. Await this future through
/// cancellation so daemon cleanup completes; do not attach its lifetime to an
/// HTTP request or drop it to enforce a separate outer timeout.
pub async fn verify_update_checkpoint(
    state: &JournalDirectory,
    owner: &InstallationLock,
    operation: &UpdateOperation,
    process: &dyn ProcessRunner,
    budget: &CheckpointRestoreBudget,
    cancellation: &CancellationToken,
) -> Result<RestoredUpdateCheckpoint, UpdateError> {
    if !(Duration::from_secs(1)..=Duration::from_secs(14_400)).contains(&budget.timeout)
        || !(64 * 1024 * 1024..=64 * 1024 * 1024 * 1024).contains(&budget.maximum_bytes)
    {
        return Err(UpdateError::Validation(
            "Invalid checkpoint restore time or storage budget.".into(),
        ));
    }
    let before = evidence::check(state, owner, operation, cancellation).await?;
    let protector = super::keys::load(&before, process, cancellation).await?;
    let home = tempfile::Builder::new()
        .prefix("citadel-restore-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .map_err(|_| failed())?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + budget.timeout,
    };
    let raw = docker
        .json(&[
            "container",
            "inspect",
            &before.registration.database_container_id,
        ])
        .await?;
    let database = single(&raw)?;
    if database["Id"] != before.registration.database_container_id {
        return Err(failed());
    }
    let image = database["Image"]
        .as_str()
        .filter(|s| s.strip_prefix("sha256:").is_some_and(is_sha256))
        .ok_or_else(failed)?;
    let raw = docker.json(&["image", "inspect", image]).await?;
    let actual = single(&raw)?;
    if actual["Id"] != image
        || actual["Os"] != "linux"
        || actual["Architecture"] != operation.plan.source.architecture
        || actual["Config"]["Volumes"]
            .as_object()
            .is_none_or(|v| v.len() != 1 || !v.contains_key("/var/lib/postgresql"))
    {
        return Err(failed());
    }
    let sandbox = Sandbox {
        nonce: Uuid::now_v7(),
        image: image.into(),
        dump: before.receipt.bundle.join("database/citadel.dump"),
        maximum_bytes: budget.maximum_bytes,
    };
    // Persist before any daemon mutation. A partial/ambiguous attempt cannot be replayed.
    before.stage.create("restore-intent.json", &sandbox)?;
    let result = async {
        let id = sandbox.create(&docker).await?;
        before.stage.create("restore-container.json", &id)?;
        let restored = sandbox.restore(&docker, &id, budget.timeout).await?;
        crate::persistence::postgres::updates::validate_checkpoint_database(
            &restored,
            &before.journal,
        )?;
        let decrypted = super::keys::verify(&sandbox, &docker, &id, &protector).await?;
        Ok::<_, UpdateError>((id, decrypted))
    }
    .await;
    // Cancellation of a Docker CLI does not stop its exec/container. Cleanup
    // has an independent bounded owner and must finish before returning.
    let cleanup_token = CancellationToken::new();
    let cleanup = LocalDocker {
        process,
        cancellation: &cleanup_token,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(60),
    };
    if sandbox.cleanup(&cleanup).await.is_err() {
        return Err(UpdateError::Unavailable(format!(
            "Cannot confirm cleanup of checkpoint verifier {}. Preserve update state and inspect this exact container before recovery.",
            sandbox.name()
        )));
    }
    let (sandbox_id, decrypted_values) = result?;
    before.stage.verify_location()?;
    let after = evidence::check(state, owner, operation, cancellation).await?;
    if before.receipt != after.receipt
        || before.registration != after.registration
        || before.journal != after.journal
        || after.stage.read::<Sandbox>("restore-intent.json")? != sandbox
        || after.stage.read::<String>("restore-container.json")? != sandbox_id
    {
        return Err(failed());
    }
    let receipt = RestoredUpdateCheckpoint {
        format: 2,
        operation_id: operation.id,
        fence: operation.fence,
        instance_id: operation.instance_id,
        bundle_checksums_sha256: before.receipt.bundle_checksums_sha256,
        catalog_sha256: operation.plan.source.catalog_sha256.clone(),
        database_image: sandbox.image,
        sandbox_id,
        decrypted_values,
    };
    after.stage.create("restore-verified.json", &receipt)?;
    Ok(receipt)
}
