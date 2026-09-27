//! Recovery selects work; collection and commits belong to the scoped workers.
use super::{EventRefresh, ReconciliationScope};
use crate::PlatformKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryReason {
    Bootstrap,
    Online,
    Reconnect,
    AdminResync,
    Safety,
    SwarmSafety,
}

pub struct RuntimeRecoveryCoordinator;
impl RuntimeRecoveryCoordinator {
    /// Platform identity must be validated before scheduling dependent inventory.
    pub fn request(reason: RecoveryReason, kind: PlatformKind) -> Option<EventRefresh> {
        match reason {
            RecoveryReason::SwarmSafety => {
                (kind == PlatformKind::DockerSwarm).then_some(EventRefresh::Swarm)
            }
            _ => Some(EventRefresh::Platform),
        }
    }

    pub fn platform_committed(kind: PlatformKind) -> impl Iterator<Item = EventRefresh> {
        use ReconciliationScope::*;
        // Container persistence tolerates missing images; image commits repair
        // that relation. No ordering is needed between these independent scopes.
        [Images, Containers, Networks, Volumes]
            .into_iter()
            .map(EventRefresh::Resource)
            .chain((kind == PlatformKind::DockerSwarm).then_some(EventRefresh::Swarm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_validates_identity_first_and_composes_independent_scopes() {
        for reason in [
            RecoveryReason::Bootstrap,
            RecoveryReason::Online,
            RecoveryReason::Reconnect,
            RecoveryReason::AdminResync,
            RecoveryReason::Safety,
        ] {
            assert_eq!(
                RuntimeRecoveryCoordinator::request(reason, PlatformKind::Docker),
                Some(EventRefresh::Platform)
            );
        }
        let scopes: Vec<_> =
            RuntimeRecoveryCoordinator::platform_committed(PlatformKind::Docker).collect();
        assert_eq!(scopes.len(), 4);
        assert!(!scopes.contains(&EventRefresh::Swarm));
        assert_eq!(
            RuntimeRecoveryCoordinator::platform_committed(PlatformKind::DockerSwarm).count(),
            5
        );
        assert_eq!(
            RuntimeRecoveryCoordinator::request(RecoveryReason::SwarmSafety, PlatformKind::Docker),
            None
        );
        assert_eq!(
            RuntimeRecoveryCoordinator::request(
                RecoveryReason::SwarmSafety,
                PlatformKind::DockerSwarm
            ),
            Some(EventRefresh::Swarm)
        );
    }
}
