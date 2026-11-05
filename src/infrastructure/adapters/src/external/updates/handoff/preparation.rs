//! Target trust, compatibility and configuration checks completed before shutdown.
use super::super::{
    CheckedCoreReplacement, InstallerComposeInputs, bootstrap::verify_handoff_source,
    docker::LocalDocker,
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
    descriptor: Option<UpgradeDescriptor>,
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
        let mut execution = self.check_preparation_phase()?;
        let agents = self.check_checkpoint_access(process, cancellation).await?;
        if agents != self.intent.operation.plan.agent_requirements {
            return Err(UpdateError::Conflict("Registered Agent versions changed since the update review. Review again before updating.".into()));
        }
        let plan = &self.intent.operation.plan;
        let reg = &self.intent.registration;
        let inputs = InstallerComposeInputs {
            directory: reg.directory.clone(),
            project: reg.project.clone(),
            compose_files: reg.compose_files.clone(),
        };
        let (descriptor, configuration) = if let Some(database) = &plan.database {
            super::super::compose::validate_database_update(database)?;
            self.prepare_database(process, cancellation).await?;
            (
                None,
                inputs
                    .prepare_database_replacement(database, process, cancellation)
                    .await?
                    .checked,
            )
        } else {
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
            (
                Some(target.descriptor),
                inputs
                    .check_replacement(&plan.target.image, process, cancellation)
                    .await?,
            )
        };
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
        verification::acknowledged_helper(&docker, &self.intent, &self.acknowledgement).await?;
        verify_handoff_source(
            &docker,
            reg,
            Some(&self.acknowledgement.binding.helper_container_id),
        )
        .await?;
        if self.check_checkpoint_access(process, cancellation).await? != agents {
            return Err(conflict());
        }
        if self.check_preparation_phase()? != execution {
            return Err(conflict());
        }
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        execution.prepared = Some(PreparedTarget {
            descriptor,
            configuration,
            agent_versions: agents,
            checked_at: chrono::Utc::now(),
        });
        execution.save(&self.state, &self.owner)
    }

    fn check_preparation_phase(&self) -> Result<ExecutionJournal, UpdateError> {
        self.state.verify_lock(&self.owner)?;
        self.intent.verify_saved(&self.state)?;
        let execution = ExecutionJournal::read_for(&self.state, &self.intent)?;
        let journal = &execution.journal;
        if journal.operation.phase != UpdatePhase::Draining
            || journal.operation.revision != 1
            || self
                .state
                .read_optional::<HelperAcknowledgement>(&self.intent.file("shutdown-grant"))?
                .is_some()
            || execution.prepared.is_some()
            || execution.acknowledgement != self.acknowledgement
        {
            return Err(conflict());
        }
        Ok(execution)
    }
}

impl ExecutionJournal {
    pub(super) fn prepared_target(
        &self,
        intent: &HelperLaunchIntent,
        fresh: bool,
    ) -> Result<&CheckedCoreReplacement, UpdateError> {
        let receipt = self.prepared.as_ref().ok_or_else(conflict)?;
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
        match (&plan.database, &receipt.descriptor) {
            (Some(database), None) if plan.source == plan.target => {
                super::super::compose::validate_database_update(database)?
            }
            (None, Some(descriptor)) => descriptor.validate_transition(
                &plan.source,
                &plan.target,
                &receipt.agent_versions,
            )?,
            _ => return Err(conflict()),
        }
        Ok(&receipt.configuration)
    }
}

#[cfg(test)]
pub(super) fn record_fixture(state: &super::super::JournalDirectory, intent: &HelperLaunchIntent) {
    let plan = &intent.operation.plan;
    let mut execution = ExecutionJournal::read(state, intent.operation.id).unwrap();
    execution.prepared = Some(PreparedTarget {
        descriptor: Some(UpgradeDescriptor {
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
        }),
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
