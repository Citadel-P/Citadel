//! Connection-local permission decisions, valid only for the bound Phase 14
//! actor generation. Safety authentication clears them before revalidation.
use super::RealtimeReadError;
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use citadel_runtime::runtime_metrics::RuntimeWork;
use std::{collections::BTreeMap, sync::Mutex};
use tokio::sync::watch;
use uuid::Uuid;

type Key = (ResourceType, Option<Uuid>, i32, Option<i32>);
const MAX_DECISIONS: usize = 1024;

pub struct AuthorizationLease {
    actor: ActorId,
    generation: Option<watch::Receiver<Uuid>>,
    decisions: Mutex<BTreeMap<Key, bool>>,
}
impl AuthorizationLease {
    pub(super) fn new(actor: ActorId, generation: Option<watch::Receiver<Uuid>>) -> Self {
        Self {
            actor,
            generation,
            decisions: Mutex::new(BTreeMap::new()),
        }
    }
    pub(super) fn tracked(&self) -> bool {
        self.generation.is_some()
    }
    pub(super) fn clear(&self) {
        self.decisions.lock().unwrap().clear();
    }
    fn current(&self, principal: &ActorPrincipal) -> Result<(), RealtimeReadError> {
        if principal.actor_id != self.actor
            || self
                .generation
                .as_ref()
                .is_some_and(|generation| generation.has_changed().unwrap_or(true))
        {
            return Err(RealtimeReadError::Authorization);
        }
        Ok(())
    }
    pub(crate) async fn daemon_capabilities(
        &self,
        identity: &IdentityService,
        principal: &ActorPrincipal,
        id: Uuid,
    ) -> Result<
        (
            crate::api::resources::platforms::views::PlatformCapabilitiesView,
            crate::api::resources::platforms::views::VolumeCapabilitiesView,
        ),
        RealtimeReadError,
    > {
        use crate::api::resources::platforms::views::{
            PlatformCapabilitiesView, VolumeCapabilitiesView,
        };
        use PermissionLevel::{Execute, Read, Write};
        use SpecificPermission::*;
        self.authorize(
            identity,
            principal,
            ResourceType::Platform,
            Some(id),
            Read,
            None,
        )
        .await?;
        let allowed = |kind, level, specific| async move {
            match self
                .authorize(identity, principal, kind, Some(id), level, specific)
                .await
            {
                Ok(()) => Ok(true),
                Err(RealtimeReadError::Authorization) => {
                    self.current(principal)?;
                    Ok(false)
                }
                Err(error) => Err(error),
            }
        };
        let platform = PlatformCapabilitiesView {
            can_read: true,
            can_write: allowed(ResourceType::Platform, Write, None).await?,
            can_execute: allowed(ResourceType::Platform, Execute, None).await?,
            can_view_logs: allowed(ResourceType::Platform, Read, Some(Logs)).await?,
            can_inspect: allowed(ResourceType::Platform, Read, Some(Inspect)).await?,
            can_open_terminal: allowed(ResourceType::Platform, Read, Some(Terminal)).await?,
            can_pull: allowed(ResourceType::Platform, Read, Some(Pull)).await?,
            can_manage_node_agents: allowed(
                ResourceType::Platform,
                Execute,
                Some(ManageNodeAgents),
            )
            .await?,
        };
        let volume = VolumeCapabilitiesView {
            can_read: platform.can_read,
            can_write: platform.can_write,
            can_execute: platform.can_execute,
            can_inspect: platform.can_inspect,
            can_browse: allowed(ResourceType::Volume, Read, Some(Browse)).await?,
            can_download: allowed(ResourceType::Volume, Read, Some(Download)).await?,
        };
        Ok((platform, volume))
    }

    pub(crate) async fn authorize(
        &self,
        identity: &IdentityService,
        principal: &ActorPrincipal,
        kind: ResourceType,
        id: Option<Uuid>,
        level: PermissionLevel,
        specific: Option<SpecificPermission>,
    ) -> Result<(), RealtimeReadError> {
        let _lookup = RuntimeWork::RealtimePermissionLookup.start();
        self.current(principal)?;
        if principal.is_administrator() {
            return Ok(());
        }
        let key = (kind, id, level as i32, specific.map(|s| s as i32));
        if self.tracked() {
            let decisions = self.decisions.lock().unwrap();
            let decision = decisions.get(&key).copied().or_else(|| {
                decisions
                    .get(&(kind, None, key.2, key.3))
                    .copied()
                    .filter(|allowed| *allowed)
            });
            if let Some(allowed) = decision {
                RuntimeWork::RealtimePermissionLookup.units(1);
                return if allowed {
                    Ok(())
                } else {
                    Err(RealtimeReadError::Authorization)
                };
            }
            // Fail closed rather than retain unbounded authorization state or
            // repeatedly thrash decisions at runtime-event frequency.
            if decisions.len() >= MAX_DECISIONS {
                return Err(RealtimeReadError::Authorization);
            }
        }
        let _miss = RuntimeWork::RealtimePermissionMiss.start();
        let result = match id {
            Some(id) => {
                identity
                    .authorize_resource(principal, kind, id, level, specific)
                    .await
            }
            None => identity.authorize(principal, kind, level, specific).await,
        };
        self.current(principal)?;
        let allowed = match result {
            Ok(()) => true,
            Err(IdentityError::Forbidden) => false,
            Err(error) => return Err(RealtimeReadError::Storage(error.to_string())),
        };
        if self.tracked() {
            self.decisions.lock().unwrap().insert(key, allowed);
        }
        if allowed {
            Ok(())
        } else {
            Err(RealtimeReadError::Authorization)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn principal(actor: ActorId) -> ActorPrincipal {
        ActorPrincipal {
            subject_id: Uuid::now_v7(),
            actor_id: actor,
            name: "lease".into(),
            principal_type: citadel_identity::AuthenticatedPrincipalType::User,
            credential_id: None,
            roles: vec![],
        }
    }
    #[test]
    fn actor_fences_are_targeted_and_closed_watches_fail_closed() {
        let alice = principal(ActorId::new(Uuid::now_v7()));
        let bob = principal(ActorId::new(Uuid::now_v7()));
        let (alice_changes, alice_watch) = watch::channel(Uuid::now_v7());
        let (bob_changes, bob_watch) = watch::channel(Uuid::now_v7());
        let alice_lease = AuthorizationLease::new(alice.actor_id, Some(alice_watch));
        let bob_lease = AuthorizationLease::new(bob.actor_id, Some(bob_watch));
        assert!(alice_lease.current(&alice).is_ok());
        assert!(alice_lease.current(&bob).is_err());
        alice_changes.send_replace(Uuid::now_v7());
        assert!(alice_lease.current(&alice).is_err());
        assert!(bob_lease.current(&bob).is_ok());
        // Clearing decisions must never acknowledge an invalid generation.
        alice_lease.clear();
        assert!(alice_lease.current(&alice).is_err());
        drop(bob_changes);
        assert!(bob_lease.current(&bob).is_err());
    }
}
