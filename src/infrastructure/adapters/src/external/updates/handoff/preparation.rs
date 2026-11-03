//! Target trust, compatibility and configuration checks completed before shutdown.
use super::super::{
    CheckedCoreReplacement, InstallerComposeInputs, JournalDirectory,
    bootstrap::verify_handoff_source, docker::LocalDocker,
};
use super::ExecutionJournal;
use super::{
    OwnedUpdateHelper,
    evidence::{HelperAcknowledgement, HelperLaunchIntent, conflict},
    runtime::private_home,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{ReleaseInspectionService, UpdateError, UpdatePhase, UpgradeDescriptor};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreparedTarget {
    descriptor: UpgradeDescriptor,
    configuration: CheckedCoreReplacement,
    agent_versions: Vec<String>,
    checked_at: chrono::DateTime<chrono::Utc>,
}

impl OwnedUpdateHelper {
    /// This is required before Core may issue its shutdown grant. It does not
    /// stop Core, mutate installer inputs, or grant execution on its own.
    pub async fn prepare_target(
        &self,
        inspector: &ReleaseInspectionService,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        self.check_preparation_phase()?;
        let agents = self.check_checkpoint_access(process, cancellation).await?;
        if agents != self.intent.operation.plan.agent_requirements {
            return Err(UpdateError::Conflict("Registered Agent versions changed since the update review. Review again before updating.".into()));
        }
        let plan = &self.intent.operation.plan;
        let target = inspector
            .inspect(
                &plan.target.version,
                &plan.target.architecture,
                cancellation,
            )
            .await?;
        if target.identity != plan.target {
            return Err(conflict());
        }
        target
            .descriptor
            .validate_transition(&plan.source, &plan.target, &agents)?;
        let reg = &self.intent.registration;
        let configuration = InstallerComposeInputs {
            directory: reg.directory.clone(),
            project: reg.project.clone(),
            compose_files: reg.compose_files.clone(),
        }
        .check_replacement(&plan.target.image, process, cancellation)
        .await?;
        if configuration.source_configuration_sha256 != reg.configuration_sha256 {
            return Err(conflict());
        }
        // Signature inspection can involve a slow pull. Repeat live and database
        // checks before publishing evidence instead of retaining an early snapshot.
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(120),
        };
        let helper =
            verification::helper(&docker, &self.intent, &self.acknowledgement.binding).await?;
        if helper.started_at != self.acknowledgement.started_at {
            return Err(conflict());
        }
        verify_handoff_source(
            &docker,
            reg,
            Some(&self.acknowledgement.binding.helper_container_id),
        )
        .await?;
        if self.check_checkpoint_access(process, cancellation).await? != agents {
            return Err(conflict());
        }
        self.check_preparation_phase()?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let mut execution = ExecutionJournal::read(&self.state, self.intent.operation.id)?;
        execution.prepared = Some(PreparedTarget {
            descriptor: target.descriptor,
            configuration,
            agent_versions: agents,
            checked_at: chrono::Utc::now(),
        });
        execution.save(&self.state, &self.owner)
    }

    fn check_preparation_phase(&self) -> Result<(), UpdateError> {
        self.state.verify_lock(&self.owner)?;
        self.intent.verify_saved(&self.state)?;
        let journal = self.state.journal(self.intent.operation.id)?;
        journal.validate_for(&self.intent.operation)?;
        if journal.operation.phase != UpdatePhase::Draining
            || journal.operation.revision != 1
            || self
                .state
                .read_optional::<HelperAcknowledgement>(&self.intent.file("shutdown-grant"))?
                .is_some()
            || ExecutionJournal::read(&self.state, self.intent.operation.id)?
                .prepared
                .is_some()
        {
            return Err(conflict());
        }
        Ok(())
    }
}

pub(super) fn check_prepared_target(
    state: &JournalDirectory,
    intent: &HelperLaunchIntent,
    acknowledgement: &HelperAcknowledgement,
    fresh: bool,
) -> Result<CheckedCoreReplacement, UpdateError> {
    let execution = ExecutionJournal::read(state, intent.operation.id)?;
    if execution.acknowledgement != *acknowledgement {
        return Err(conflict());
    }
    let receipt = execution.prepared.ok_or_else(conflict)?;
    let plan = &intent.operation.plan;
    let now = chrono::Utc::now();
    if receipt.configuration.source_configuration_sha256 != plan.configuration_sha256
        || receipt.configuration.target_image != plan.target.image
        || receipt.configuration.target_configuration_sha256 == plan.configuration_sha256
        || !citadel_updates::is_sha256(&receipt.configuration.target_configuration_sha256)
        || receipt.agent_versions != plan.agent_requirements
        || receipt.checked_at < intent.operation.created_at
        || receipt.checked_at > now
        || (fresh && now - receipt.checked_at > chrono::Duration::minutes(15))
    {
        return Err(conflict());
    }
    receipt
        .descriptor
        .validate_transition(&plan.source, &plan.target, &receipt.agent_versions)?;
    Ok(receipt.configuration)
}

#[cfg(test)]
pub(super) fn record_fixture(state: &JournalDirectory, intent: &HelperLaunchIntent) {
    let plan = &intent.operation.plan;
    let mut execution = ExecutionJournal::read(state, intent.operation.id).unwrap();
    execution.prepared = Some(PreparedTarget {
        descriptor: UpgradeDescriptor {
            schema_version: 1,
            product: "citadel".into(),
            target_version: plan.target.version.clone(),
            target_catalog_sha256: plan.target.catalog_sha256.clone(),
            installer_format: citadel_updates::INSTALLER_FORMAT,
            update_protocol: citadel_updates::UPDATE_PROTOCOL,
            sources: vec![citadel_updates::UpgradeSource {
                version: plan.source.version.clone(),
                catalog_sha256: plan.source.catalog_sha256.clone(),
                architectures: vec![plan.source.architecture.clone()],
                agent_versions: vec![],
            }],
        },
        configuration: CheckedCoreReplacement {
            source_configuration_sha256: plan.configuration_sha256.clone(),
            target_configuration_sha256: "c".repeat(64),
            target_image: plan.target.image.clone(),
        },
        agent_versions: vec![],
        checked_at: chrono::Utc::now(),
    });
    state
        .write(&format!("{}.json", intent.operation.id), &execution)
        .unwrap();
}
