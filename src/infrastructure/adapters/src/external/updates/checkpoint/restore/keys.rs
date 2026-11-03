//! Recovery keys stay in the executor; only ciphertext leaves isolated PostgreSQL.
use super::super::super::{
    InstallerComposeInputs, check_recovery_keys, docker::LocalDocker, recovery_keys::read_key,
};
use super::{
    evidence::{Evidence, failed},
    sandbox::{DATABASE, Sandbox},
};
use crate::{
    persistence::postgres::updates::{
        CHECKPOINT_SECRET_PAGE_SIZE, checkpoint_secret_page, checkpoint_secrets_query,
    },
    security::identity::crypto::AesGcmSecretProtector,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_execution::ProcessRunner;
use citadel_updates::UpdateError;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

pub(super) async fn load(
    evidence: &Evidence,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<AesGcmSecretProtector, UpdateError> {
    let external = &evidence.receipt.required_external_configuration;
    if external.iter().any(|s| {
        !matches!(
            s.as_str(),
            "CITADEL_JWT_KEY" | "CITADEL_SECRETS_ENCRYPTION_KEY"
        )
    }) || external.len()
        != external
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
    {
        return Err(failed());
    }
    let mut jwt = None;
    let mut encryption = None;
    if !external.is_empty() {
        let r = &evidence.registration;
        let inputs = InstallerComposeInputs {
            directory: r.directory.clone(),
            project: r.project.clone(),
            compose_files: r.compose_files.clone(),
        };
        let (checked, mut config) = inputs.resolve(process, cancellation).await?;
        if checked.configuration_sha256 != evidence.receipt.configuration_sha256 {
            return Err(failed());
        }
        for (name, destination) in [
            ("CITADEL_JWT_KEY", &mut jwt),
            ("CITADEL_SECRETS_ENCRYPTION_KEY", &mut encryption),
        ] {
            if external.iter().any(|key| key == name) {
                let serde_json::Value::String(value) =
                    config["services"]["server"]["environment"][name].take()
                else {
                    return Err(failed());
                };
                if value.trim().is_empty() {
                    return Err(failed());
                }
                *destination = Some(Zeroizing::new(value));
            }
        }
    }
    let root = evidence.receipt.bundle.join("recovery");
    check_recovery_keys(
        &root,
        jwt.as_ref().map(|s| s.as_str()),
        encryption.as_ref().map(|s| s.as_str()),
    )?;
    let encoded = match encryption {
        Some(value) => Zeroizing::new(value.as_bytes().to_vec()),
        None => read_key(&root.join("secret-encryption-key"))?,
    };
    let key = Zeroizing::new(
        STANDARD
            .decode(std::str::from_utf8(&encoded).map_err(|_| failed())?.trim())
            .map_err(|_| failed())?,
    );
    AesGcmSecretProtector::new(&key).map_err(|_| failed())
}

pub(super) async fn verify(
    sandbox: &Sandbox,
    docker: &LocalDocker<'_>,
    id: &str,
    protector: &AesGcmSecretProtector,
) -> Result<u64, UpdateError> {
    let mut cursor = (0, Uuid::nil());
    let mut count = 0_u64;
    loop {
        let query = checkpoint_secrets_query(cursor);
        let bytes = Zeroizing::new(
            docker
                .run(&[
                    "exec",
                    id,
                    "psql",
                    "--no-psqlrc",
                    "--host=/var/run/postgresql",
                    "--username=postgres",
                    "--dbname",
                    DATABASE,
                    "--tuples-only",
                    "--no-align",
                    "--set=ON_ERROR_STOP=1",
                    "--command",
                    &query,
                ])
                .await?,
        );
        let rows = checkpoint_secret_page(&bytes, cursor)?;
        for row in &rows {
            // Authenticated decryption with the existing production protector.
            // The returned plaintext is zeroized immediately and never serialized.
            drop(protector.unprotect(&row.envelope).map_err(|_| UpdateError::Unavailable(
                "Recovery encryption key cannot decrypt all restored credentials. Preserve the original key and checkpoint; replacement was not authorized.".into()
            ))?);
            cursor = (row.kind, row.id);
            count = count.checked_add(1).ok_or_else(failed)?;
        }
        if rows.len() < CHECKPOINT_SECRET_PAGE_SIZE {
            break;
        }
    }
    let after = sandbox.inspect(docker, id).await?;
    if after["State"]["Running"] != true || after["State"]["OOMKilled"] != false {
        return Err(failed());
    }
    Ok(count)
}
