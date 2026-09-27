//! Transport-neutral daemon-event policy. Recovery controls have a separate path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerChange {
    Created,
    Running,
    Paused,
    Exited,
    Observe,
    Tombstone,
}

impl ContainerChange {
    pub const fn state(self) -> Option<&'static str> {
        match self {
            Self::Created => Some("created"),
            Self::Running => Some("running"),
            Self::Paused => Some("paused"),
            Self::Exited => Some("exited"),
            Self::Observe | Self::Tombstone => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceChange {
    Observe,
    Tombstone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmResource {
    Network,
    Node,
    Service,
    Secret,
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEventKind {
    Container(ContainerChange),
    Image(ResourceChange),
    Volume(ResourceChange),
    Network(ResourceChange),
    SwarmDirty(SwarmResource),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReconciliationScope {
    Containers,
    Images,
    Networks,
    Volumes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaOutcome {
    Pending,
    Applied,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationDecision {
    None,
    ApplyDelta,
    Reconcile(ReconciliationScope),
    SwarmDirty,
}

/// Unknown notifications cannot establish uncertainty in every resource. Recovery
/// after actual stream loss is owned by the subscription, never by this policy.
pub fn event_decision(
    kind: RuntimeEventKind,
    swarm_target: bool,
    swarm_scope: bool,
    delta: DeltaOutcome,
) -> ReconciliationDecision {
    use ReconciliationDecision as Decision;
    use RuntimeEventKind as Event;
    match kind {
        Event::Unknown => Decision::None,
        Event::SwarmDirty(_) => {
            if swarm_target {
                Decision::SwarmDirty
            } else {
                Decision::None
            }
        }
        Event::Network(_) if swarm_target && swarm_scope => Decision::SwarmDirty,
        Event::Container(_) if delta == DeltaOutcome::Applied && swarm_target => {
            Decision::SwarmDirty
        }
        Event::Container(_) | Event::Image(_) | Event::Network(_) | Event::Volume(_) => match delta
        {
            DeltaOutcome::Pending => Decision::ApplyDelta,
            DeltaOutcome::Applied => Decision::None,
            DeltaOutcome::Unavailable => Decision::Reconcile(match kind {
                Event::Container(_) => ReconciliationScope::Containers,
                Event::Image(_) => ReconciliationScope::Images,
                Event::Network(_) => ReconciliationScope::Networks,
                Event::Volume(_) => ReconciliationScope::Volumes,
                _ => unreachable!(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_deltas_request_only_the_matching_resource() {
        for (kind, scope) in [
            (
                RuntimeEventKind::Container(ContainerChange::Exited),
                ReconciliationScope::Containers,
            ),
            (
                RuntimeEventKind::Image(ResourceChange::Observe),
                ReconciliationScope::Images,
            ),
            (
                RuntimeEventKind::Network(ResourceChange::Tombstone),
                ReconciliationScope::Networks,
            ),
            (
                RuntimeEventKind::Volume(ResourceChange::Observe),
                ReconciliationScope::Volumes,
            ),
        ] {
            assert_eq!(
                event_decision(kind, false, false, DeltaOutcome::Pending),
                ReconciliationDecision::ApplyDelta
            );
            assert_eq!(
                event_decision(kind, false, false, DeltaOutcome::Applied),
                ReconciliationDecision::None
            );
            assert_eq!(
                event_decision(kind, false, false, DeltaOutcome::Unavailable),
                ReconciliationDecision::Reconcile(scope)
            );
        }
    }
    #[test]
    fn swarm_changes_never_escalate_to_platform_inventory() {
        assert_eq!(
            event_decision(
                RuntimeEventKind::SwarmDirty(SwarmResource::Service),
                true,
                true,
                DeltaOutcome::Pending
            ),
            ReconciliationDecision::SwarmDirty
        );
        assert_eq!(
            event_decision(
                RuntimeEventKind::Network(ResourceChange::Observe),
                true,
                true,
                DeltaOutcome::Pending
            ),
            ReconciliationDecision::SwarmDirty
        );
        assert_eq!(
            event_decision(
                RuntimeEventKind::Container(ContainerChange::Exited),
                true,
                false,
                DeltaOutcome::Applied
            ),
            ReconciliationDecision::SwarmDirty
        );
        assert_eq!(
            event_decision(
                RuntimeEventKind::Unknown,
                true,
                true,
                DeltaOutcome::Unavailable
            ),
            ReconciliationDecision::None
        );
    }
}
