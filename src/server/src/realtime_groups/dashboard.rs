//! Dashboard subscriptions use the same bounded, actor-filtered projections as HTTP.
use super::{ApplicationGroupReader, ClientEvent, Group, GroupSnapshot};
use crate::api::resources::{
    activities::{authorized::activity_access, views::RecentActivityView},
    dashboards::{
        DashboardFleetQuery, DashboardSourceQuery, DashboardSourceView, SavedDashboardView,
    },
};
use crate::realtime::{PublishedRuntimeEvent, RealtimeReadError};
use citadel_activities::ActivityEventType;
use citadel_dashboards::{DashboardError, DashboardViewer, FleetFilter, FleetRead, SourceFilter};
use citadel_identity::ActorPrincipal;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Projection {
    Board(Uuid),
    Fleet(FleetFilter),
    Sources(SourceFilter),
    Recent {
        tags: Vec<String>,
        event_type: Option<ActivityEventType>,
        limit: i32,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Join {
    group: String,
    subscription_id: Uuid,
    #[serde(default)]
    tags: Vec<String>,
    projection: String,
    query: Value,
}

pub(super) fn parse(value: &Value) -> Option<Group> {
    let req: Join = serde_json::from_value(value.clone()).ok()?;
    if req.subscription_id.is_nil() {
        return None;
    }
    let tags = citadel_tags::tag_filter_names(req.tags).ok()?;
    let mut g = Group::parse(&req.group)?;
    g.dashboard = Some(match req.projection.as_str() {
        "getDashboard" => {
            Projection::Board(serde_json::from_value(req.query.get("id")?.clone()).ok()?)
        }
        "getDashboardFleet" => Projection::Fleet(
            serde_json::from_value::<DashboardFleetQuery>(req.query)
                .ok()?
                .filter(tags)
                .ok()?,
        ),
        "listDashboardSources" => Projection::Sources(
            serde_json::from_value::<DashboardSourceQuery>(req.query)
                .ok()?
                .filter(tags)
                .ok()?,
        ),
        "listRecentActivities" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
            struct Recent {
                limit: i32,
                event_type: Option<ActivityEventType>,
            }
            let q: Recent = serde_json::from_value(req.query).ok()?;
            if !(1..=10).contains(&q.limit) {
                return None;
            }
            Projection::Recent {
                tags,
                event_type: q.event_type,
                limit: q.limit,
            }
        }
        _ => return None,
    });
    g.subscription_id = Some(req.subscription_id);
    g.name = req.subscription_id.to_string();
    Some(g)
}

impl Projection {
    pub(super) fn affected_by(&self, e: &PublishedRuntimeEvent) -> bool {
        let kind = e.resource_type;
        if let Self::Board(id) = self {
            return kind == "Dashboard" && e.affects_resource(Some(*id));
        }
        let tags = match self {
            Self::Fleet(f) => &f.tags,
            Self::Sources(f) => &f.tags,
            Self::Recent { tags, .. } => tags,
            Self::Board(_) => unreachable!(),
        };
        if matches!(kind, "Tag" | "TagUsage" | "ResourceTags") {
            return !tags.is_empty();
        }
        if matches!(self, Self::Recent { .. }) {
            return kind == "Activity";
        }
        if e.event_kind == "buildLogs" {
            return false;
        }
        if e.event_kind == "runtimeChanged"
            && !matches!(
                e.payload["dockerResourceType"].as_str(),
                Some(
                    "container"
                        | "platform"
                        | "platformInventory"
                        | "swarm"
                        | "task"
                        | "service"
                        | "image"
                )
            )
        {
            return false;
        }
        match self {
            Self::Sources(f) => {
                let source = match f.kind {
                    citadel_dashboards::SourceKind::Platform => "Platform",
                    citadel_dashboards::SourceKind::Deployment => "Deployment",
                    citadel_dashboards::SourceKind::Stack => "Stack",
                    citadel_dashboards::SourceKind::SwarmService => "SwarmService",
                };
                (kind == "Platform" && source != "Platform")
                    || (kind == source
                        && (f.ids.is_empty()
                            || f.ids.iter().any(|id| e.affects_resource(Some(*id)))))
            }
            Self::Fleet(f) => {
                if kind == "Alert" {
                    return matches!(
                        f.read,
                        FleetRead::Summary | FleetRead::Attention | FleetRead::OpenAlerts
                    );
                }
                matches!(
                    kind,
                    "Platform"
                        | "Deployment"
                        | "Stack"
                        | "SwarmService"
                        | "Build"
                        | "BuildAgentPool"
                        | "Registry"
                        | "GitRepository"
                        | "Script"
                        | "BackupPolicy"
                ) && (kind == "Platform" || f.kinds.is_empty() || f.kinds.iter().any(|k| k == kind))
            }
            _ => false,
        }
    }
    pub(super) async fn read(
        &self,
        reader: &ApplicationGroupReader,
        p: &ActorPrincipal,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let viewer = DashboardViewer {
            actor: p.actor_id,
            administrator: p.is_administrator(),
        };
        let failure = |e: DashboardError| RealtimeReadError::Storage(e.to_string());
        let mut scope = None;
        let data = match self {
            Self::Board(id) => match reader.dashboards.get(viewer, *id).await {
                Ok(row) => json!(SavedDashboardView::from(row)),
                Err(DashboardError::Unavailable | DashboardError::Forbidden) => Value::Null,
                Err(e) => return Err(failure(e)),
            },
            Self::Fleet(f) => json!(
                reader
                    .dashboards
                    .fleet(viewer, f.clone())
                    .await
                    .map_err(failure)?
            ),
            Self::Sources(f) => {
                let sources = reader
                    .dashboards
                    .sources(viewer, f.clone())
                    .await
                    .map_err(failure)?;
                // A missing pin (or an unpinned search) can gain a member on any platform.
                if !f.ids.is_empty() && sources.len() == f.ids.len() {
                    scope = sources
                        .iter()
                        .map(|source| source.platform_id)
                        .collect::<Option<_>>()
                        .map(super::SnapshotScope::PinnedPlatforms);
                }
                json!(
                    sources
                        .into_iter()
                        .map(DashboardSourceView::from)
                        .collect::<Vec<_>>()
                )
            }
            Self::Recent {
                tags,
                event_type,
                limit,
            } => json!(
                reader
                    .activities
                    .recent(&activity_access(p), *event_type, *limit, tags)
                    .await
                    .map_err(|e| RealtimeReadError::Storage(e.to_string()))?
                    .into_iter()
                    .map(RecentActivityView::from)
                    .collect::<Vec<_>>()
            ),
        };
        Ok(GroupSnapshot {
            scope,
            rows: vec![],
            events: vec![ClientEvent::new("DashboardSnapshot", vec![data])],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime_groups::GroupSubscription;

    fn scope(projection: &str, query: Value) -> Value {
        json!({"group":"dashboard","subscriptionId":Uuid::now_v7(),"projection":projection,"query":query})
    }

    #[test]
    fn dashboard_scopes_enforce_http_limits_and_skip_telemetry() {
        assert!(
            parse(&scope(
                "getDashboardFleet",
                json!({"read":"Members","limit":26})
            ))
            .is_none()
        );
        assert!(parse(&scope("listRecentActivities", json!({"limit":11}))).is_none());
        let id = Uuid::now_v7();
        let group = parse(&scope(
            "getDashboardFleet",
            json!({"read":"Attention","kinds":"Deployment"}),
        ))
        .unwrap();
        let mut e = PublishedRuntimeEvent {
            platform_id: Some(id),
            resource_type: "Platform",
            resource_id: id,
            event_kind: "runtimeChanged",
            resource_revision: 1,
            reads: Default::default(),
            containers: Default::default(),
            payload: json!({"dockerResourceType":"containerStats"}),
        };
        assert!(!group.affected_by(&e));
        e.resource_type = "Alert";
        e.event_kind = "resourceChanged";
        assert!(group.affected_by(&e));
        e.resource_type = "Script";
        assert!(!group.affected_by(&e));
        let recent = parse(&scope("listRecentActivities", json!({"limit":5}))).unwrap();
        assert!(!recent.affected_by(&e));
        e.resource_type = "Activity";
        assert!(recent.affected_by(&e));
    }

    #[test]
    fn repeated_authorized_results_do_not_produce_socket_messages() {
        let mut joined = GroupSubscription::new(
            parse(&scope("listRecentActivities", json!({"limit":5}))).unwrap(),
        );
        let snapshot = |value| GroupSnapshot {
            scope: None,
            events: vec![ClientEvent::new("DashboardSnapshot", vec![value])],
            rows: vec![],
        };
        assert_eq!(joined.apply(snapshot(json!([])), 50).unwrap().len(), 1);
        assert!(joined.apply(snapshot(json!([])), 50).unwrap().is_empty());
        assert_eq!(
            joined
                .apply(snapshot(json!([{"id":"visible"}])), 50)
                .unwrap()
                .len(),
            1
        );
    }
}
