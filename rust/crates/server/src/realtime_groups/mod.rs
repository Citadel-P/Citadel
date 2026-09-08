mod reader;
pub use reader::ApplicationGroupReader;

use crate::realtime::{PublishedRuntimeEvent, RealtimeReadError};
use citadel_identity::ActorPrincipal;
use futures_util::future::BoxFuture;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const MAX_GROUPS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub kind: String,
    pub id: Option<Uuid>,
    pub reference: Option<String>,
}

impl Group {
    pub fn parse(name: &str) -> Option<Self> {
        if name.is_empty()
            || name.len() > 256
            || name.trim() != name
            || name.chars().any(char::is_control)
        {
            return None;
        }
        let parts: Vec<_> = name.split(':').collect();
        let mut group = Self {
            name: name.into(),
            kind: parts[0].into(),
            id: None,
            reference: None,
        };
        match parts.as_slice() {
            [
                "platforms"
                | "deployments"
                | "stacks"
                | "git-repositories"
                | "automation-actions"
                | "backup-repositories"
                | "backup-policies"
                | "build-projects"
                | "build-agent-pools"
                | "alert-events",
            ] => {}
            [
                "containers"
                | "images"
                | "docker-daemon"
                | "deployment"
                | "stack"
                | "stack-info"
                | "swarm-service"
                | "swarm-services"
                | "git-repo"
                | "automation-action"
                | "backup-policy"
                | "backup-repository"
                | "backup-runs"
                | "backup-run"
                | "backup-restore-runs"
                | "backup-restore-run"
                | "build-project"
                | "build-agent-pool"
                | "build-runs"
                | "build-run",
                id,
            ] => {
                group.id = Some(Uuid::parse_str(id).ok()?);
            }
            ["container-info", reference] if !reference.is_empty() && reference.len() <= 128 => {
                group.reference = Some((*reference).into());
            }
            ["container-log", reference]
                if Uuid::parse_str(reference).is_ok_and(|id| !id.is_nil())
                    || (matches!(reference.len(), 12 | 64)
                        && reference.bytes().all(|b| b.is_ascii_hexdigit())) =>
            {
                group.reference = Some((*reference).into());
            }
            ["stack-log", id] => {
                group.id = Some(Uuid::parse_str(id).ok()?);
            }
            ["container-exec", reference, session] if valid_session(session) => {
                Group::parse(&format!("container-log:{reference}"))?;
                group.reference = Some((*reference).into());
            }
            ["swarm-task-exec", platform, task, session]
                if valid_session(session) && !task.is_empty() && task.len() <= 128 =>
            {
                group.id = Some(Uuid::parse_str(platform).ok()?);
                group.reference = Some((*task).into());
            }
            ["activity", kind, id] => {
                group.id = Some(Uuid::parse_str(id).ok()?);
                group.reference = Some((*kind).into());
            }
            _ => return None,
        }
        if group.id.is_some_and(|id| id.is_nil()) {
            return None;
        }
        Some(group)
    }

    pub fn affected_by(&self, event: &PublishedRuntimeEvent) -> bool {
        if self.kind.ends_with("-log") || self.kind.ends_with("-exec") {
            return false;
        }
        if event.event_kind == "buildLogs" {
            return self.kind == "build-run" && self.id == Some(event.resource_id);
        }
        if self.kind == "activity" {
            // The existing Alert router publishes a coarse Alert invalidation
            // for Rule mutations too. Reload through the authorized activity
            // reader; never broadcast audit payloads directly to subscribers.
            let resource_matches = self.reference.as_deref() == Some(event.resource_type)
                || (self.reference.as_deref() == Some("AlertRule")
                    && event.resource_type == "Alert");
            return event.payload["dockerResourceType"] != "containerStats"
                && resource_matches
                && (event.resource_id.is_nil() || self.id == Some(event.resource_id));
        }
        if let Some(platform) = event.platform_id {
            if event.payload["dockerResourceType"] == "containerStats" {
                return matches!(
                    self.kind.as_str(),
                    "platforms" | "container-info" | "stack-info"
                ) || (self.kind == "containers" && self.id == Some(platform));
            }
            return match self.kind.as_str() {
                "platforms" | "container-info" | "stack-info" => true,
                "containers" | "docker-daemon" | "images" | "swarm-services" => {
                    self.id == Some(platform)
                }
                _ => false,
            };
        }
        match self.kind.as_str() {
            "platforms" => matches!(
                event.resource_type,
                "Platform" | "Deployment" | "Stack" | "SwarmService" | "ResourceTags"
            ),
            "deployment" | "deployments" => {
                matches!(event.resource_type, "Deployment" | "ResourceTags")
            }
            "stack" | "stacks" | "stack-info" => {
                matches!(event.resource_type, "Stack" | "ResourceTags")
            }
            "swarm-service" | "swarm-services" => event.resource_type == "SwarmService",
            "git-repo" | "git-repositories" => event.resource_type == "GitRepository",
            "automation-action" | "automation-actions" => event.resource_type == "AutomationAction",
            "backup-policy"
            | "backup-policies"
            | "backup-runs"
            | "backup-run"
            | "backup-restore-runs"
            | "backup-restore-run" => event.resource_type == "BackupPolicy",
            "backup-repository" | "backup-repositories" => {
                event.resource_type == "BackupRepository"
            }
            "build-project" | "build-projects" | "build-runs" | "build-run" => {
                event.resource_type == "Build"
            }
            "build-agent-pool" | "build-agent-pools" => event.resource_type == "BuildAgentPool",
            "alert-events" => event.resource_type == "Alert",
            _ => false,
        }
    }
}

fn valid_session(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}

pub struct TerminalInvocation {
    pub group: Group,
    pub action: TerminalAction,
}
pub enum TerminalAction {
    Start(citadel_platforms::terminal::TerminalShell),
    Input(citadel_platforms::terminal::TerminalInput),
}
pub struct GroupTerminal {
    pub input: citadel_platforms::terminal::TerminalInputSender,
    pub output: GroupStream,
}

#[derive(Debug, Serialize)]
pub struct ClientEvent {
    #[serde(rename = "protocolVersion")]
    version: u16,
    kind: &'static str,
    pub target: &'static str,
    pub arguments: Vec<Value>,
}
impl ClientEvent {
    pub fn new(target: &'static str, arguments: Vec<Value>) -> Self {
        Self {
            version: 1,
            kind: "event",
            target,
            arguments,
        }
    }
}

pub struct GroupRows {
    pub target: &'static str,
    pub rows: Vec<Value>,
    pub style: RowStyle,
}
#[derive(Clone, Copy)]
pub enum RowStyle {
    Update,
    Daemon,
    DockerResource,
    Activity,
    Notification,
    Platforms,
}
#[derive(Default)]
pub struct GroupSnapshot {
    pub rows: Vec<GroupRows>,
    pub events: Vec<ClientEvent>,
}
pub trait GroupReadPort: Send + Sync {
    fn terminal_invocation<'a>(
        &'a self,
        _p: &'a ActorPrincipal,
        _method: &'a str,
        _args: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<TerminalInvocation>, RealtimeReadError>> {
        Box::pin(async { Ok(None) })
    }
    fn terminal<'a>(
        &'a self,
        _p: &'a ActorPrincipal,
        _group: &'a Group,
        _shell: citadel_platforms::terminal::TerminalShell,
        _cancel: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<GroupTerminal, RealtimeReadError>> {
        Box::pin(async { Err(RealtimeReadError::Authorization) })
    }
    fn read<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        group: &'a Group,
        event: Option<&'a PublishedRuntimeEvent>,
    ) -> BoxFuture<'a, Result<GroupSnapshot, RealtimeReadError>>;
    fn stream<'a>(
        &'a self,
        _principal: &'a ActorPrincipal,
        _group: &'a Group,
        _cancel: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<Option<GroupStream>, RealtimeReadError>> {
        Box::pin(async { Ok(None) })
    }
    fn invocation_group<'a>(
        &'a self,
        _principal: &'a ActorPrincipal,
        _target: &'a str,
        _arguments: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<Group>, RealtimeReadError>> {
        Box::pin(async { Ok(None) })
    }
}
pub type GroupStream =
    futures_util::stream::BoxStream<'static, Result<ClientEvent, RealtimeReadError>>;

pub struct GroupSubscription {
    pub group: Group,
    known: BTreeMap<&'static str, BTreeSet<String>>,
    // Only routing fields needed to identify deleted Containers/Images, never
    // their configuration, environment, or secret-bearing resource payloads.
    tombstones: BTreeMap<&'static str, BTreeMap<String, Value>>,
}
impl GroupSubscription {
    pub fn new(group: Group) -> Self {
        Self {
            group,
            known: BTreeMap::new(),
            tombstones: BTreeMap::new(),
        }
    }
    pub fn apply(
        &mut self,
        snapshot: GroupSnapshot,
        limit: usize,
    ) -> Result<Vec<ClientEvent>, RealtimeReadError> {
        let mut events = snapshot.events;
        for batch in snapshot.rows {
            if batch.rows.len() > limit {
                return Err(RealtimeReadError::Storage(
                    "Realtime snapshot limit exceeded".into(),
                ));
            }
            let ids: BTreeSet<_> = batch
                .rows
                .iter()
                .filter_map(|row| {
                    row["id"]
                        .as_str()
                        .or_else(|| row["name"].as_str())
                        .map(str::to_owned)
                })
                .collect();
            let initial = !self.known.contains_key(batch.target);
            let previous = self.known.entry(batch.target).or_default();
            for removed in previous.difference(&ids) {
                match batch.style {
                    RowStyle::Platforms => {
                        events.push(ClientEvent::new("PlatformsDeleted", vec![json!(removed)]))
                    }
                    RowStyle::Activity | RowStyle::Notification => {}
                    RowStyle::DockerResource => {
                        if let Some(row) = self
                            .tombstones
                            .get_mut(batch.target)
                            .and_then(|rows| rows.remove(removed))
                        {
                            let action = if batch.target == "ImageEventReceived" {
                                "delete"
                            } else {
                                "destroy"
                            };
                            events.push(ClientEvent::new(batch.target, vec![row, json!(action)]));
                        }
                    }
                    RowStyle::Daemon => events.push(ClientEvent::new(
                        batch.target,
                        vec![Value::Null, json!("destroy"), json!(removed)],
                    )),
                    RowStyle::Update => events.push(ClientEvent::new(
                        batch.target,
                        vec![json!({"id":removed}), json!("delete")],
                    )),
                }
            }
            for row in batch.rows {
                let id = row["id"]
                    .as_str()
                    .or_else(|| row["name"].as_str())
                    .unwrap_or_default();
                let args = match batch.style {
                    RowStyle::Platforms => vec![row],
                    RowStyle::Activity => {
                        if previous.contains(id) {
                            continue;
                        }
                        vec![row]
                    }
                    RowStyle::Notification => {
                        if initial || previous.contains(id) {
                            continue;
                        }
                        vec![row]
                    }
                    RowStyle::Daemon => vec![row.clone(), json!("create"), json!(id)],
                    RowStyle::DockerResource => {
                        let tombstone = [
                            "id",
                            "containerId",
                            "dockerImageId",
                            "stackId",
                            "deploymentId",
                            "dockerNodeId",
                        ]
                        .into_iter()
                        .filter_map(|key| row.get(key).map(|v| (key.to_owned(), v.clone())))
                        .collect::<serde_json::Map<_, _>>();
                        self.tombstones
                            .entry(batch.target)
                            .or_default()
                            .insert(id.into(), Value::Object(tombstone));
                        vec![row, json!("update")]
                    }
                    // "update" upserts in the existing hooks and avoids duplicate
                    // creates when the initial REST read and group join overlap.
                    RowStyle::Update => vec![row, json!("update")],
                };
                events.push(ClientEvent::new(batch.target, args));
            }
            *previous = ids;
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alert_notifications_do_not_replay_history_or_repeat_existing_events() {
        let mut subscription = GroupSubscription::new(Group::parse("alert-events").unwrap());
        let snapshot = |ids: &[&str]| GroupSnapshot {
            rows: vec![GroupRows {
                target: "AlertEventReceived",
                rows: ids.iter().map(|id| json!({"id":id})).collect(),
                style: RowStyle::Notification,
            }],
            events: vec![],
        };
        assert!(
            subscription
                .apply(snapshot(&["old"]), 100)
                .unwrap()
                .is_empty()
        );
        let events = subscription.apply(snapshot(&["old", "new"]), 100).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].arguments, vec![json!({"id":"new"})]);
        assert!(
            subscription
                .apply(snapshot(&["old", "new"]), 100)
                .unwrap()
                .is_empty()
        );
        assert!(subscription.apply(snapshot(&[]), 100).unwrap().is_empty());
    }
    #[test]
    fn alert_rule_activity_follows_committed_alert_invalidations_only() {
        let id = Uuid::now_v7();
        let group = Group::parse(&format!("activity:AlertRule:{id}")).unwrap();
        let mut event = PublishedRuntimeEvent {
            platform_id: None,
            resource_type: "Alert",
            resource_id: Uuid::nil(),
            event_kind: "resourceChanged",
            resource_revision: 1,
            payload: json!({}),
        };
        assert!(group.affected_by(&event));
        assert!(
            !Group::parse(&format!("activity:User:{id}"))
                .unwrap()
                .affected_by(&event)
        );
        event.resource_id = Uuid::now_v7();
        assert!(!group.affected_by(&event));
        event.resource_id = id;
        event.resource_type = "AlertRule";
        assert!(group.affected_by(&event));
        event.payload = json!({"dockerResourceType":"containerStats"});
        assert!(!group.affected_by(&event));
    }
    #[test]
    fn rejects_private_untyped_malformed_and_unimplemented_stream_groups() {
        for name in [
            "",
            " platforms",
            "platforms\n",
            "alert-events:private",
            "activity:bad",
            "containers:bad",
            "container-log:abc",
            "container-exec:abc:session",
        ] {
            assert!(Group::parse(name).is_none(), "{name:?}");
        }
        assert!(Group::parse("platforms").is_some());
        assert!(Group::parse(&format!("containers:{}", Uuid::now_v7())).is_some());
    }
    #[test]
    fn snapshots_upsert_and_remove_without_retaining_resource_payloads() {
        let mut subscription = GroupSubscription::new(Group::parse("deployments").unwrap());
        let snapshot = |rows| GroupSnapshot {
            rows: vec![GroupRows {
                target: "DeploymentInfoUpdated",
                rows,
                style: RowStyle::Update,
            }],
            events: vec![],
        };
        let first = subscription
            .apply(snapshot(vec![json!({"id":"one","name":"web"})]), 10)
            .unwrap();
        assert_eq!(first[0].arguments[1], "update");
        let removed = subscription.apply(snapshot(vec![]), 10).unwrap();
        assert_eq!(
            removed[0].arguments,
            json!([{"id":"one"},"delete"]).as_array().unwrap().clone()
        );
        assert!(subscription.apply(snapshot(vec![]), 10).unwrap().is_empty());
    }
    #[test]
    fn oversized_snapshots_fail_closed() {
        let mut subscription = GroupSubscription::new(Group::parse("platforms").unwrap());
        assert!(
            subscription
                .apply(
                    GroupSnapshot {
                        rows: vec![GroupRows {
                            target: "PlatformUpdated",
                            rows: vec![json!({"id":"a"}), json!({"id":"b"})],
                            style: RowStyle::Platforms
                        }],
                        events: vec![]
                    },
                    1
                )
                .is_err()
        );
    }

    #[test]
    fn dotnet_deployment_and_service_discriminators_survive_named_event_serialization() {
        let registry = Uuid::now_v7();
        let deployment: citadel_deployments::DeploymentSpec = serde_json::from_value(json!({
            "image":{"$type":"External","registryId":registry,"imageTag":"nginx:latest","resolvedDigest":"sha256:applied"},
            "updateBehavior":"Notify"
        })).unwrap();
        let image = citadel_swarm_services::SwarmServiceImageInfo::External {
            registry_id: registry,
            image_tag: "redis".into(),
            resolved_digest: Some("sha256:applied".into()),
        };
        for (target, spec) in [
            (
                "DeploymentInfoUpdated",
                serde_json::to_value(deployment).unwrap(),
            ),
            ("SwarmServiceInfoUpdated", json!({"image":image})),
        ] {
            let encoded = serde_json::to_value(ClientEvent::new(
                target,
                vec![json!({"spec":spec}), json!("update")],
            ))
            .unwrap();
            assert_eq!(
                encoded["arguments"][0]["spec"]["image"]["$type"],
                "External"
            );
            assert_eq!(
                encoded["arguments"][0]["spec"]["image"]["resolvedDigest"],
                "sha256:applied"
            );
            assert_eq!(encoded["arguments"][1], "update");
        }
    }

    #[test]
    fn deleted_container_keeps_routing_identity_without_retaining_configuration() {
        let mut subscription = GroupSubscription::new(
            Group::parse(&format!("docker-daemon:{}", Uuid::now_v7())).unwrap(),
        );
        let snapshot = |rows| GroupSnapshot {
            rows: vec![GroupRows {
                target: "ContainerEventReceived",
                style: RowStyle::DockerResource,
                rows,
            }],
            events: vec![],
        };
        subscription.apply(snapshot(vec![json!({"id":"projection","containerId":"docker-id","stackId":"stack","environment":["SECRET=value"]})]),10).unwrap();
        let deleted = subscription.apply(snapshot(vec![]), 10).unwrap();
        assert_eq!(
            deleted[0].arguments,
            vec![
                json!({"id":"projection","containerId":"docker-id","stackId":"stack"}),
                json!("destroy")
            ]
        );
        assert!(subscription.tombstones.values().all(BTreeMap::is_empty));
    }
}
