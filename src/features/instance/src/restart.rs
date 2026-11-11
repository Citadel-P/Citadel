use crate::{InstanceError, InstanceRestartRepository};
use citadel_identity::{ActorPrincipal, Clock};
use std::sync::Arc;
use uuid::Uuid;

pub struct RestartStatus {
    pub available: bool,
    pub boot_id: Uuid,
    pub pending: bool,
}

/// Durable restart admission. Hosting owns shutdown/relaunch and startup acknowledgement.
pub struct InstanceRestartService {
    repository: Arc<dyn InstanceRestartRepository>,
    clock: Arc<dyn Clock>,
    boot_id: Uuid,
    ready: std::sync::atomic::AtomicBool,
    changes: tokio::sync::watch::Sender<()>,
}

impl InstanceRestartService {
    pub fn new(repository: Arc<dyn InstanceRestartRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            repository,
            clock,
            boot_id: Uuid::now_v7(),
            ready: std::sync::atomic::AtomicBool::new(false),
            changes: tokio::sync::watch::channel(()).0,
        }
    }
    pub async fn status(&self, principal: &ActorPrincipal) -> Result<RestartStatus, InstanceError> {
        Self::authorize(principal)?;
        Ok(RestartStatus {
            available: true,
            boot_id: self.boot_id,
            pending: self.pending().await?,
        })
    }
    pub async fn request(
        &self,
        principal: &ActorPrincipal,
        boot_id: Uuid,
    ) -> Result<(), InstanceError> {
        Self::authorize(principal)?;
        if !self.ready.load(std::sync::atomic::Ordering::Acquire) {
            return Err(InstanceError::Conflict(
                "Core startup is still completing.".into(),
            ));
        }
        if boot_id != self.boot_id {
            return Err(InstanceError::Conflict(
                "Core has already restarted. Refresh its status before requesting another restart."
                    .into(),
            ));
        }
        self.repository
            .request(principal, self.boot_id, self.clock.now())
            .await?;
        self.changes.send_replace(());
        Ok(())
    }
    pub fn changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.changes.subscribe()
    }
    pub async fn pending(&self) -> Result<bool, InstanceError> {
        self.repository.pending(self.boot_id).await
    }
    pub async fn complete_startup(&self) -> Result<(), InstanceError> {
        self.repository.complete(self.boot_id).await?;
        self.ready.store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
    fn authorize(principal: &ActorPrincipal) -> Result<(), InstanceError> {
        if !principal.is_administrator()
            || principal.principal_type != citadel_identity::AuthenticatedPrincipalType::User
        {
            return Err(InstanceError::Forbidden);
        }
        Ok(())
    }
}
