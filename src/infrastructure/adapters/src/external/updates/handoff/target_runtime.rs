//! Target-side Docker transport; normal Core never opens the private host journal.
use super::super::docker::{LocalDocker, single};
use super::{
    command::{HelperAction, HelperRequest, HelperResponse},
    completion::TargetUpdateCompletion,
    evidence::{HelperAcknowledgement, HelperBinding, conflict},
    runtime::private_home,
    target::TargetUpdateEvidence,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateOperation};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub struct TargetUpdateSession {
    pub evidence: TargetUpdateEvidence,
    helper: HelperAcknowledgement,
}

impl TargetUpdateSession {
    pub async fn connect(
        operation: &UpdateOperation,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<Self, UpdateError> {
        let hostname =
            std::fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| conflict())?;
        Self::connect_from(operation, process, cancellation, hostname.trim()).await
    }

    pub(super) async fn connect_from(
        operation: &UpdateOperation,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
        hostname: &str,
    ) -> Result<Self, UpdateError> {
        if hostname.len() != 12 || !hostname.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(conflict());
        }
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(180),
        };
        let ids = docker
            .containers(&format!(
                "label={}={}",
                verification::OPERATION,
                operation.id
            ))
            .await?;
        let [id] = ids.as_slice() else {
            return Err(conflict());
        };
        let raw = docker.json(&["container", "inspect", id]).await?;
        let helper = single(&raw)?;
        if helper["Config"]["Image"] != operation.plan.helper_image
            || helper["State"]["Running"] != true
            || helper["RestartCount"] != 0
        {
            return Err(conflict());
        }
        let user = helper["Config"]["User"].as_str().ok_or_else(conflict)?;
        if user
            .split_once(':')
            .is_none_or(|(uid, gid)| uid.parse::<u32>().is_err() || gid.parse::<u32>().is_err())
        {
            return Err(conflict());
        }
        let acknowledgement = HelperAcknowledgement {
            binding: HelperBinding {
                nonce: helper["Config"]["Labels"][verification::NONCE]
                    .as_str()
                    .ok_or_else(conflict)?
                    .parse()
                    .map_err(|_| conflict())?,
                helper_container_id: id.clone(),
            },
            started_at: helper["State"]["StartedAt"]
                .as_str()
                .ok_or_else(conflict)?
                .into(),
        };
        let HelperResponse::Ready(evidence) = (HelperRequest {
            operation: operation.clone(),
            binding: acknowledgement.binding.clone(),
            started_at: acknowledgement.started_at.clone(),
            action: HelperAction::InspectTarget,
        })
        .exchange(&docker, user, Duration::from_secs(150))
        .await?
        else {
            return Err(conflict());
        };
        let evidence = *evidence;
        let intent = &evidence.intent;
        evidence.journal.validate_for(operation)?;
        if evidence.journal.history.first() != Some(&intent.operation)
            || intent.operation.id != operation.id
            || intent.operation.fence != operation.fence
            || intent.operation.plan != operation.plan
            || evidence.journal.helper_container_id != *id
        {
            return Err(conflict());
        }
        intent.registration.validate()?;
        verification::acknowledged_helper(&docker, intent, &acknowledgement).await?;
        if !evidence.container_id.starts_with(hostname) {
            return Err(conflict());
        }
        let target = verification::container(&docker, &evidence.container_id).await?;
        if target["Config"]["Hostname"] != hostname
            || target["State"]["StartedAt"] != evidence.started_at
            || target["Config"]["Image"] != operation.plan.target.image
            || target["Image"]
                != verification::release_image(&docker, &operation.plan.target).await?
        {
            return Err(conflict());
        }
        Ok(Self {
            evidence,
            helper: acknowledgement,
        })
    }

    /// Repeat the read-only runtime proof around migrations and database verification.
    pub async fn revalidate(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let current = Self::connect(&self.evidence.intent.operation, process, cancellation).await?;
        if current.evidence != self.evidence || current.helper != self.helper {
            return Err(conflict());
        }
        Ok(())
    }

    /// The receipt must have been committed by target-owned persistence first.
    pub async fn acknowledge(
        &self,
        completion: &TargetUpdateCompletion,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(180),
        };
        let intent = &self.evidence.intent;
        let bound = &self.helper.binding;
        verification::acknowledged_helper(&docker, intent, &self.helper).await?;
        let user = format!(
            "{}:{}",
            intent.registration.executor_uid, intent.registration.executor_gid
        );
        if (HelperRequest {
            operation: intent.operation.clone(),
            binding: bound.clone(),
            started_at: self.helper.started_at.clone(),
            action: HelperAction::CompleteTarget(Box::new(completion.clone())),
        })
        .exchange(&docker, &user, Duration::from_secs(150))
        .await?
            != HelperResponse::Ready(Box::new(self.evidence.clone()))
        {
            return Err(conflict());
        }
        Ok(())
    }

    /// A missing, restarted, killed or failed executor remains ambiguous. No
    /// container is removed here, so its exit evidence remains inspectable.
    pub async fn wait_for_exit(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(180),
        };
        loop {
            if verification::helper_exited(&docker, &self.evidence.intent, &self.helper).await? {
                return Ok(());
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
                () = tokio::time::sleep(Duration::from_millis(250)) => (),
            }
        }
    }
}
