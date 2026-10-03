//! Docker's transport events reduced to the semantics shared by Local and Agent/Edge.
//! Classification is pure: ignored events and tombstones never require observation I/O.

use super::projection::DockerEvent;

pub use citadel_platforms::jobs::{
    ContainerChange, ResourceChange, RuntimeEventKind, SwarmResource,
};

/// Preserve IDs, timestamps and attributes in the caller; only the bounded semantic
/// kind belongs here. `None` means no inventory change and no observation is needed.
pub fn normalize(event: &DockerEvent) -> Option<RuntimeEventKind> {
    classify(&event.resource_type, &event.action, &event.scope)
}

/// Also used at the protobuf boundary so older Agents obey the same semantics.
pub fn classify(resource: &str, action: &str, scope: &str) -> Option<RuntimeEventKind> {
    use RuntimeEventKind as Event;
    if action.is_empty() {
        return None;
    }
    Some(match resource {
        "container" => Event::Container(match action {
            a if a.starts_with("exec_") => return None,
            // Health details, OOM flags, terminal geometry and resource limits are
            // not fields in the persisted container projection. Exit is owned by die.
            a if a.starts_with("health_status") => return None,
            "attach" | "top" | "kill" | "stop" | "oom" | "resize" | "update" => return None,
            // Docker restart emits die + start, then this completion marker. Retain
            // the real transitions (including a failed restart's die), not a third
            // observation. FIFO processing makes the final start win.
            "restart" => return None,
            "die" => ContainerChange::Exited,
            "pause" => ContainerChange::Paused,
            "start" | "unpause" => ContainerChange::Running,
            "create" => ContainerChange::Created,
            "destroy" => ContainerChange::Tombstone,
            _ => ContainerChange::Observe,
        }),
        "image" => Event::Image(match action {
            "create" | "pull" => ResourceChange::Observe,
            "delete" => ResourceChange::Tombstone,
            _ => return None,
        }),
        "volume" => Event::Volume(match action {
            "create" => ResourceChange::Observe,
            "destroy" => ResourceChange::Tombstone,
            _ => return None,
        }),
        "network" => match action {
            "create" => Event::Network(ResourceChange::Observe),
            "destroy" => Event::Network(ResourceChange::Tombstone),
            _ if scope == "swarm" => Event::SwarmDirty(SwarmResource::Network),
            _ => return None,
        },
        "node" => Event::SwarmDirty(SwarmResource::Node),
        "service" => Event::SwarmDirty(SwarmResource::Service),
        // Tasks invalidate service relationships; the wire contract has no task kind.
        "task" => Event::SwarmDirty(SwarmResource::Service),
        "secret" => Event::SwarmDirty(SwarmResource::Secret),
        "config" => Event::SwarmDirty(SwarmResource::Config),
        "builder" | "plugin" => return None,
        _ => Event::Unknown,
    })
}

#[cfg(test)]
mod tests;
