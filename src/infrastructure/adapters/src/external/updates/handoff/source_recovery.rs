//! Retire one failed executor without restoring an untouched source database.
use super::super::{
    JournalDirectory, RegisteredComposeInstallation, bootstrap::verify_recoverable_source,
    docker::LocalDocker,
};
use super::{
    ExecutionJournal, HelperLaunchIntent,
    evidence::{HelperBinding, conflict},
    runtime::private_home,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateOperation, UpdatePhase};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub(in crate::external::updates) async fn recover_source(
    operation: &UpdateOperation,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    if !matches!(
        operation.phase,
        UpdatePhase::Preparing
            | UpdatePhase::Draining
            | UpdatePhase::Failed
            | UpdatePhase::Postponed
    ) {
        return Err(conflict());
    }
    let state = JournalDirectory::open(operation.plan.checkpoint_directory.clone().into())?;
    let name = format!("checkpoint-{}", operation.id);
    match rustix::fs::mkdirat(
        state.directory(),
        name.as_str(),
        rustix::fs::Mode::from_raw_mode(0o700),
    ) {
        Ok(()) => state.sync()?,
        Err(rustix::io::Errno::EXIST) => {}
        Err(_) => return Err(conflict()),
    }
    let archive = JournalDirectory::open(state.path().join(name))?;
    let registration: RegisteredComposeInstallation = state.read("installation.json")?;
    registration.validate()?;
    if registration.executor_uid != rustix::process::geteuid().as_raw()
        || registration.executor_gid != rustix::process::getegid().as_raw()
        || registration.docker_socket != std::path::Path::new("/var/run/docker.sock")
        || registration.instance_id != operation.instance_id
        || registration.id != operation.plan.registration_id
        || registration.configuration_sha256 != operation.plan.configuration_sha256
        || registration.state_directory != state.path()
        || registration.fingerprint(std::path::Path::new("/"))?
            != operation.plan.configuration_sha256
    {
        return Err(conflict());
    }
    let launch = format!("launch-{}.json", operation.id);
    let intent: Option<HelperLaunchIntent> = match state.read_optional(&launch)? {
        Some(value) => Some(value),
        None => archive.read_optional(&launch)?,
    };
    if let Some(intent) = &intent
        && (intent.registration != registration
            || intent.operation.id != operation.id
            || intent.operation.fence != operation.fence
            || intent.operation.plan != operation.plan
            || intent.operation.actor_id != operation.actor_id
            || intent.operation.idempotency_key != operation.idempotency_key)
    {
        return Err(conflict());
    }
    let binding_name = format!("binding-{}.json", operation.id);
    let saved_binding: Option<HelperBinding> = match state.read_optional(&binding_name)? {
        Some(value) => Some(value),
        None => archive.read_optional(&binding_name)?,
    };
    if let Some(binding) = &saved_binding
        && binding.nonce != intent.as_ref().ok_or_else(conflict)?.nonce
    {
        return Err(conflict());
    }
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(120),
    };
    let helpers = docker
        .containers(&format!(
            "label={}={}",
            verification::OPERATION,
            operation.id
        ))
        .await?;
    if helpers.len() > 1 {
        return Err(conflict());
    }
    if let Some(binding) = &saved_binding {
        let exact = docker
            .containers(&format!("id={}", binding.helper_container_id))
            .await?;
        if exact != helpers {
            return Err(conflict());
        }
    }
    if let Some(id) = helpers.first() {
        let intent = intent.as_ref().ok_or_else(conflict)?;
        let binding = HelperBinding {
            nonce: intent.nonce,
            helper_container_id: id.clone(),
        };
        let actual = verification::helper_container(&docker, intent, &binding).await?;
        if actual["State"]["Running"] == true {
            // Stop only the verified helper. Its write-ahead stop/replacement
            // evidence is checked under its released OS lock below.
            docker
                .run(&["container", "stop", "--time", "30", id])
                .await?;
        }
        let actual = verification::helper_container(&docker, intent, &binding).await?;
        if actual["State"]["Running"] != false
            || !matches!(
                actual["State"]["Status"].as_str(),
                Some("created" | "exited")
            )
        {
            return Err(conflict());
        }
    }
    let owner = state.existing_lock()?;
    let journal_name = format!("{}.json", operation.id);
    let execution: Option<ExecutionJournal> = match state.read_optional(&journal_name)? {
        Some(value) => Some(value),
        None => archive.read_optional(&journal_name)?,
    };
    if let Some(execution) = &execution {
        execution
            .journal
            .validate_for(&intent.as_ref().ok_or_else(conflict)?.operation)?;
        if saved_binding.as_ref() != Some(&execution.acknowledgement.binding)
            || execution.stop.is_some()
            || execution.replacement.is_some()
            || execution.target.is_some()
            || execution.journal.history.iter().any(|entry| {
                !matches!(
                    entry.phase,
                    UpdatePhase::Preparing
                        | UpdatePhase::Draining
                        | UpdatePhase::Failed
                        | UpdatePhase::Postponed
                )
            })
        {
            return Err(conflict());
        }
    } else if operation.checkpoint.is_some()
        || !matches!(
            operation.phase,
            UpdatePhase::Preparing | UpdatePhase::Failed
        )
    {
        return Err(conflict());
    }
    // No source stop was issued. Same-container restarts may fail readiness while
    // in maintenance; identity/configuration/storage still have to match exactly.
    verify_recoverable_source(&docker, &registration, helpers.first().map(String::as_str)).await?;
    let image = verification::release_image(&docker, &operation.plan.source).await?;
    if verification::container(&docker, &registration.core_container_id).await?["Image"] != image {
        return Err(conflict());
    }
    // This permanent tombstone fences late launch, acknowledgement and grants.
    // It is outside active state, so ordinary startup can resume after DB settlement.
    let recovery = match archive.read_optional::<UpdateOperation>("source-recovery.json")? {
        Some(saved) => {
            if saved.id != operation.id
                || saved.instance_id != operation.instance_id
                || saved.fence != operation.fence
                || saved.plan != operation.plan
                || saved.actor_id != operation.actor_id
                || saved.idempotency_key != operation.idempotency_key
                || saved.created_at != operation.created_at
                || saved.revision > operation.revision
            {
                return Err(conflict());
            }
            saved
        }
        None => {
            archive.create("source-recovery.json", operation)?;
            operation.clone()
        }
    };
    if let Some(id) = helpers.first() {
        let binding = HelperBinding {
            nonce: intent.as_ref().ok_or_else(conflict)?.nonce,
            helper_container_id: id.clone(),
        };
        let actual = verification::helper_container(
            &docker,
            intent.as_ref().ok_or_else(conflict)?,
            &binding,
        )
        .await?;
        if actual["State"]["Running"] != false {
            return Err(conflict());
        }
        match archive.read_optional::<serde_json::Value>("executor-exit.json")? {
            Some(saved) if saved != actual["State"] => return Err(conflict()),
            Some(_) => {}
            None => archive.create("executor-exit.json", &actual["State"])?,
        }
        docker.run(&["container", "rm", id]).await?;
    }
    for name in [
        launch,
        format!("binding-{}.json", operation.id),
        format!("shutdown-grant-{}.json", operation.id),
        journal_name,
    ] {
        if state.read_optional::<serde_json::Value>(&name)?.is_some() {
            state.archive(&owner, &archive, &name)?;
        }
    }
    verify_recoverable_source(&docker, &registration, None).await?;
    match archive.read_optional::<UpdateOperation>("source-recovered.json")? {
        Some(saved) if saved != recovery => return Err(conflict()),
        Some(_) => {}
        None => archive.create("source-recovered.json", &recovery)?,
    }
    state.require_registration_only()?;
    state.verify_lock(&owner)?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    Ok(())
}
