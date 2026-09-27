use crate::realtime::topic::Topic;
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
    // Preserve the client's spelling for idempotent joins/leaves, including
    // accepted alternate UUID spellings. Never canonicalize a subscription key.
    pub name: String,
    topic: crate::realtime::topic::Topic,
}
impl Group {
    pub fn parse(name: &str) -> Option<Self> {
        Some(Self {
            name: name.into(),
            topic: crate::realtime::topic::Topic::parse(name)?,
        })
    }
    pub fn topic(&self) -> &crate::realtime::topic::Topic {
        &self.topic
    }
    pub fn id(&self) -> Option<Uuid> {
        self.topic.id()
    }
    pub fn reference(&self) -> Option<&str> {
        self.topic.reference()
    }
    pub fn affected_by(&self, event: &PublishedRuntimeEvent) -> bool {
        if let Some(patches) = event
            .payload
            .get("containerPatches")
            .and_then(Value::as_array)
        {
            return match self.topic() {
                Topic::Containers(..) | Topic::DockerDaemon(..) => self.id() == event.platform_id,
                Topic::ContainerInfo(reference) => patches.iter().any(|p| {
                    p["id"]
                        .as_str()
                        .and_then(|id| Uuid::parse_str(id).ok())
                        .is_some_and(|id| Uuid::parse_str(reference).ok() == Some(id))
                }),
                _ => false,
            };
        }

        if matches!(self.topic(), Topic::ContainerLog(..) | Topic::StackLog(..))
            || matches!(
                self.topic(),
                Topic::ContainerExec { .. } | Topic::SwarmTaskExec { .. }
            )
        {
            return false;
        }
        if event.event_kind == "buildLogs" {
            return matches!(self.topic(), Topic::BuildRun(..))
                && self.id() == Some(event.resource_id);
        }
        if matches!(self.topic(), Topic::Activity { .. })
            && event.event_kind == "runtimeChanged"
            && event.payload["dockerResourceType"] != "containerStats"
            && matches!(
                self.reference(),
                Some("Deployment" | "Stack" | "SwarmService")
            )
        {
            return true;
        }
        if matches!(self.topic(), Topic::Activity { .. }) {
            // The existing Alert router publishes a coarse Alert invalidation
            // for Rule mutations too. Reload through the authorized activity
            // reader; never broadcast audit payloads directly to subscribers.
            let resource_matches = self.reference() == Some(event.resource_type)
                || (self.reference() == Some("AlertRule") && event.resource_type == "Alert");
            return event.payload["dockerResourceType"] != "containerStats"
                && resource_matches
                && event.affects_resource(self.id());
        }
        if let Some(platform) = event.platform_id {
            if event.event_kind == "updated"
                && matches!(event.resource_type, "Deployment" | "Stack")
            {
                return match self.topic() {
                    Topic::Platforms => true,
                    Topic::Deployments | Topic::Deployment(..) => {
                        event.resource_type == "Deployment" && event.affects_resource(self.id())
                    }
                    Topic::Stacks | Topic::Stack(..) | Topic::StackInfo(..) => {
                        event.resource_type == "Stack" && event.affects_resource(self.id())
                    }
                    _ => false,
                };
            }
            if matches!(self.topic(), Topic::Images(..))
                && event.payload["dockerResourceType"] == "container"
                && matches!(
                    event.payload["action"].as_str(),
                    Some("update" | "start" | "stop" | "die" | "kill" | "pause" | "unpause")
                )
            {
                return false;
            }
            if event.payload["dockerResourceType"] == "containerStats" {
                return (matches!(self.topic(), Topic::Platforms)
                    && event.payload["platformSample"].is_object())
                    || matches!(
                        self.topic(),
                        Topic::ContainerInfo(..) | Topic::StackInfo(..)
                    )
                    || (matches!(self.topic(), Topic::Containers(..))
                        && self.id() == Some(platform));
            }
            return match self.topic() {
                Topic::Platforms
                | Topic::ContainerInfo(..)
                | Topic::StackInfo(..)
                | Topic::Deployment(..)
                | Topic::Deployments
                | Topic::Stack(..)
                | Topic::Stacks
                | Topic::SwarmService(..) => true,
                Topic::Containers(..)
                | Topic::DockerDaemon(..)
                | Topic::Images(..)
                | Topic::SwarmServices(..) => self.id() == Some(platform),
                _ => false,
            };
        }
        match self.topic() {
            Topic::Platforms => matches!(
                event.resource_type,
                "Platform" | "Deployment" | "Stack" | "SwarmService" | "ResourceTags"
            ),
            Topic::Deployment(..) | Topic::Deployments => {
                event.resource_type == "ResourceTags"
                    || (event.resource_type == "Deployment" && event.affects_resource(self.id()))
            }
            Topic::Stack(..) | Topic::Stacks | Topic::StackInfo(..) => {
                event.resource_type == "ResourceTags"
                    || (event.resource_type == "Stack" && event.affects_resource(self.id()))
            }
            Topic::SwarmService(..) | Topic::SwarmServices(..) => {
                event.resource_type == "SwarmService"
            }
            Topic::GitRepo(..) | Topic::GitRepositories => event.resource_type == "GitRepository",
            Topic::AutomationAction(..) | Topic::AutomationActions => {
                event.resource_type == "AutomationAction"
            }
            Topic::BackupPolicy(..)
            | Topic::BackupPolicies
            | Topic::BackupRuns(..)
            | Topic::BackupRun(..)
            | Topic::BackupRestoreRuns(..)
            | Topic::BackupRestoreRun(..) => event.resource_type == "BackupPolicy",
            Topic::BackupRepository(..) | Topic::BackupRepositories => {
                event.resource_type == "BackupRepository"
            }
            Topic::BuildProject(..)
            | Topic::BuildProjects
            | Topic::BuildRuns(..)
            | Topic::BuildRun(..) => event.resource_type == "Build",
            Topic::BuildAgentPool(..) | Topic::BuildAgentPools => {
                event.resource_type == "BuildAgentPool"
            }
            Topic::AlertEvents => event.resource_type == "Alert",
            _ => false,
        }
    }
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
#[derive(Clone)]
pub enum RowStyle {
    Update,
    Daemon,
    DockerResource,
    // Complete state for one Docker ID, not the entire Platform inventory.
    DockerResourcePatch(String),
    DaemonPatch(String),
    Activity,
    Notification,
    Platforms,
    PlatformPatch(Uuid),
}
#[derive(Default)]
pub struct GroupSnapshot {
    pub rows: Vec<GroupRows>,
    pub events: Vec<ClientEvent>,
}
pub trait GroupReadPort: Send + Sync {
    fn read_with_lease<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        group: &'a Group,
        event: Option<&'a PublishedRuntimeEvent>,
        _lease: &'a crate::realtime::AuthorizationLease,
    ) -> BoxFuture<'a, Result<GroupSnapshot, RealtimeReadError>> {
        self.read(principal, group, event)
    }

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
            if let RowStyle::PlatformPatch(id) = batch.style {
                let known = self.known.entry(batch.target).or_default();
                if batch.rows.is_empty() {
                    if known.remove(&id.to_string()) {
                        events.push(ClientEvent::new("PlatformsDeleted", vec![json!(id)]));
                    }
                } else {
                    known.insert(id.to_string());
                    if known.len() > limit {
                        return Err(RealtimeReadError::Storage(
                            "Realtime snapshot limit exceeded".into(),
                        ));
                    }
                    for row in batch.rows {
                        events.push(ClientEvent::new(batch.target, vec![row]));
                    }
                }
                continue;
            }
            if let RowStyle::DaemonPatch(reference) = &batch.style {
                let known = self.known.entry(batch.target).or_default();
                if batch.rows.is_empty() {
                    if known.remove(reference) {
                        events.push(ClientEvent::new(
                            batch.target,
                            vec![Value::Null, json!("destroy"), json!(reference)],
                        ));
                    }
                } else {
                    for row in batch.rows {
                        known.insert(reference.clone());
                        if known.len() > limit {
                            return Err(RealtimeReadError::Storage(
                                "Realtime snapshot limit exceeded".into(),
                            ));
                        }
                        events.push(ClientEvent::new(
                            batch.target,
                            vec![row, json!("create"), json!(reference)],
                        ));
                    }
                }
                continue;
            }
            if let RowStyle::DockerResourcePatch(reference) = &batch.style {
                let known = self.known.entry(batch.target).or_default();
                let saved = self.tombstones.entry(batch.target).or_default();
                let ids: BTreeSet<_> = batch
                    .rows
                    .iter()
                    .filter_map(|row| row["id"].as_str())
                    .collect();
                let removed: Vec<_> = saved
                    .iter()
                    .filter(|(id, row)| {
                        row["containerId"].as_str() == Some(reference.as_str())
                            && !ids.contains(id.as_str())
                    })
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in removed {
                    known.remove(&id);
                    if let Some(row) = saved.remove(&id) {
                        events.push(ClientEvent::new(batch.target, vec![row, json!("destroy")]));
                    }
                }
                for row in batch.rows {
                    let Some(id) = row["id"].as_str() else {
                        continue;
                    };
                    let action = if known.insert(id.to_owned()) {
                        "create"
                    } else {
                        "update"
                    };
                    if known.len() > limit {
                        return Err(RealtimeReadError::Storage(
                            "Realtime snapshot limit exceeded".into(),
                        ));
                    }
                    saved.insert(id.to_owned(), docker_tombstone(&row));
                    events.push(ClientEvent::new(batch.target, vec![row, json!(action)]));
                }
                continue;
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
                    RowStyle::DockerResourcePatch(_)
                    | RowStyle::DaemonPatch(_)
                    | RowStyle::PlatformPatch(_) => unreachable!(),
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
                        self.tombstones
                            .entry(batch.target)
                            .or_default()
                            .insert(id.into(), docker_tombstone(&row));
                        vec![row, json!("update")]
                    }
                    RowStyle::DockerResourcePatch(_)
                    | RowStyle::DaemonPatch(_)
                    | RowStyle::PlatformPatch(_) => unreachable!(),
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

fn docker_tombstone(row: &Value) -> Value {
    Value::Object(
        [
            "id",
            "containerId",
            "dockerImageId",
            "stackId",
            "deploymentId",
            "dockerNodeId",
        ]
        .into_iter()
        .filter_map(|key| row.get(key).map(|value| (key.to_owned(), value.clone())))
        .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workload_changes_only_refresh_matching_details_and_the_list() {
        let hub = crate::realtime::RealtimeHub::new(
            8,
            std::sync::Arc::new(crate::metrics::Metrics::default()),
        );
        let mut receiver = hub.subscribe();
        let selected = Uuid::now_v7();
        let other = Uuid::now_v7();
        for (resource, detail, list) in [
            ("Deployment", "deployment", "deployments"),
            ("Stack", "stack", "stacks"),
        ] {
            hub.publish_resource_changes(resource, &[selected]);
            let event = receiver.try_recv().unwrap();
            assert!(Group::parse(list).unwrap().affected_by(&event));
            assert!(
                Group::parse(&format!("{detail}:{selected}"))
                    .unwrap()
                    .affected_by(&event)
            );
            assert!(
                !Group::parse(&format!("{detail}:{other}"))
                    .unwrap()
                    .affected_by(&event)
            );
            assert!(
                !Group::parse(&format!("activity:{resource}:{other}"))
                    .unwrap()
                    .affected_by(&event)
            );
            hub.publish_resource_change(resource, selected, "updated");
            let event = receiver.try_recv().unwrap();
            assert!(
                !Group::parse(&format!("{detail}:{other}"))
                    .unwrap()
                    .affected_by(&event)
            );
            hub.publish_resource_change(resource, Uuid::nil(), "resourceChanged");
            assert!(
                Group::parse(&format!("{detail}:{other}"))
                    .unwrap()
                    .affected_by(&receiver.try_recv().unwrap())
            );
        }
    }

    #[test]
    fn platform_patch_preserves_other_platforms_and_removes_only_its_revoked_row() {
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let mut subscription = GroupSubscription::new(Group::parse("platforms").unwrap());
        let snapshot = |rows, style| GroupSnapshot {
            rows: vec![GroupRows {
                target: "PlatformUpdated",
                rows,
                style,
            }],
            events: vec![],
        };
        subscription
            .apply(
                snapshot(vec![json!({"id":a}), json!({"id":b})], RowStyle::Platforms),
                10,
            )
            .unwrap();
        let changed = subscription
            .apply(
                snapshot(
                    vec![json!({"id":a,"containersStopped":5})],
                    RowStyle::PlatformPatch(a),
                ),
                10,
            )
            .unwrap();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].target, "PlatformUpdated");
        let revoked = subscription
            .apply(snapshot(vec![], RowStyle::PlatformPatch(a)), 10)
            .unwrap();
        assert_eq!(revoked.len(), 1);
        assert_eq!(revoked[0].arguments, vec![json!(a)]);
        let remaining = subscription
            .apply(snapshot(vec![], RowStyle::Platforms), 10)
            .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].arguments, vec![json!(b)]);
    }
    #[test]
    fn daemon_resource_patches_keep_siblings_and_replayed_deletes_are_idempotent() {
        let mut subscription = GroupSubscription::new(
            Group::parse(&format!("docker-daemon:{}", Uuid::now_v7())).unwrap(),
        );
        let snapshot = |rows, style| GroupSnapshot {
            rows: vec![GroupRows {
                target: "VolumeEventReceived",
                rows,
                style,
            }],
            events: vec![],
        };
        subscription
            .apply(
                snapshot(vec![json!({"id":"a"}), json!({"id":"b"})], RowStyle::Daemon),
                10,
            )
            .unwrap();
        let events = subscription
            .apply(snapshot(vec![], RowStyle::DaemonPatch("a".into())), 10)
            .unwrap();
        assert_eq!(
            events[0].arguments,
            vec![Value::Null, json!("destroy"), json!("a")]
        );
        assert!(subscription.known["VolumeEventReceived"].contains("b"));
        assert!(
            subscription
                .apply(snapshot(vec![], RowStyle::DaemonPatch("a".into())), 10)
                .unwrap()
                .is_empty()
        );
        let events = subscription
            .apply(
                snapshot(
                    vec![json!({"id":"a","inUse":true})],
                    RowStyle::DaemonPatch("a".into()),
                ),
                10,
            )
            .unwrap();
        assert_eq!(events[0].arguments[1], "create");
        assert_eq!(events[0].arguments[2], "a");
        assert_eq!(subscription.known["VolumeEventReceived"].len(), 2);
    }

    #[test]
    fn container_patches_preserve_other_rows_and_track_created_deleted_and_recovered_containers() {
        let mut subscription = GroupSubscription::new(
            Group::parse(&format!("docker-daemon:{}", Uuid::now_v7())).unwrap(),
        );
        let snapshot = |rows, style| GroupSnapshot {
            rows: vec![GroupRows {
                target: "ContainerEventReceived",
                rows,
                style,
            }],
            events: vec![],
        };
        let a = json!({"id":"a","containerId":"docker-a","state":"Running"});
        let b = json!({"id":"b","containerId":"docker-b","state":"Running"});
        subscription
            .apply(
                snapshot(vec![a.clone(), b.clone()], RowStyle::DockerResource),
                100,
            )
            .unwrap();
        let stopped = json!({"id":"a","containerId":"docker-a","state":"Exited"});
        let events = subscription
            .apply(
                snapshot(
                    vec![stopped.clone()],
                    RowStyle::DockerResourcePatch("docker-a".into()),
                ),
                100,
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].arguments, vec![stopped, json!("update")]);
        assert!(subscription.known["ContainerEventReceived"].contains("b"));
        let c = json!({"id":"c","containerId":"docker-c","environment":["secret"]});
        let created = subscription
            .apply(
                snapshot(vec![c], RowStyle::DockerResourcePatch("docker-c".into())),
                100,
            )
            .unwrap();
        assert_eq!(created[0].arguments[1], "create");
        let deleted = subscription
            .apply(
                snapshot(vec![], RowStyle::DockerResourcePatch("docker-c".into())),
                100,
            )
            .unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(
            deleted[0].arguments,
            vec![json!({"id":"c","containerId":"docker-c"}), json!("destroy")]
        );
        assert!(
            subscription
                .apply(
                    snapshot(vec![], RowStyle::DockerResourcePatch("docker-c".into())),
                    100
                )
                .unwrap()
                .is_empty()
        );
        let recovered = subscription
            .apply(snapshot(vec![b], RowStyle::DockerResource), 100)
            .unwrap();
        assert_eq!(
            recovered[0].arguments,
            vec![json!({"id":"a","containerId":"docker-a"}), json!("destroy")]
        );
    }
    #[test]
    fn deployment_groups_refresh_on_container_inventory_but_not_stats() {
        let mut event = PublishedRuntimeEvent {
            platform_id: Some(Uuid::now_v7()),
            resource_type: "DockerRuntime",
            resource_id: Uuid::now_v7(),
            event_kind: "runtimeChanged",
            resource_revision: 1,
            reads: Default::default(),
            containers: Default::default(),
            payload: json!({"dockerResourceType":"container"}),
        };
        for name in [
            "deployments".to_owned(),
            format!("deployment:{}", Uuid::now_v7()),
        ] {
            let group = Group::parse(&name).unwrap();
            assert!(group.affected_by(&event));
            event.payload = json!({"dockerResourceType":"containerStats"});
            assert!(!group.affected_by(&event));
            event.payload = json!({"dockerResourceType":"container"});
        }
    }
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
            reads: Default::default(),
            containers: Default::default(),
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
