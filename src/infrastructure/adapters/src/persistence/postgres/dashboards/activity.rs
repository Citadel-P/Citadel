use super::super::activities::store::insert_activity;
use citadel_activities::{ActivityEvent, ActivityEventInfo, ActivityStatus};
use citadel_dashboards::{
    DashboardConfiguration, DashboardContent, DashboardError, DashboardViewer, SavedDashboard,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    viewer: DashboardViewer,
    id: Uuid,
    name: &str,
    info: ActivityEventInfo,
) -> Result<(), DashboardError> {
    let event = ActivityEvent::new_dashboard_event(id, name.into(), viewer.actor, info)
        .map_err(|error| DashboardError::Storage(error.to_string()))?;
    insert_activity(tx, &event)
        .await
        .map_err(|error| DashboardError::Storage(error.to_string()))
}

pub(super) async fn instance_home_changed(
    tx: &mut Transaction<'_, Postgres>,
    viewer: DashboardViewer,
    dashboard_name: Option<String>,
) -> Result<(), DashboardError> {
    let now = chrono::Utc::now();
    let identity = super::super::licensing::store::get_or_create_identity(tx, now)
        .await
        .map_err(|error| DashboardError::Storage(error.to_string()))?;
    let event = ActivityEvent::new_instance_event(
        identity.instance_id,
        viewer.actor,
        ActivityEventInfo::InstanceHomeDashboardChanged { dashboard_name },
        ActivityStatus::Success,
        now,
    )
    .map_err(|error| DashboardError::Storage(error.to_string()))?;
    insert_activity(tx, &event)
        .await
        .map_err(|error| DashboardError::Storage(error.to_string()))
}

// User-provided Markdown and links can contain credentials and may outlive the board's original audience.
pub(super) fn configuration_snapshot(name: &str, configuration: &Value) -> Value {
    let mut snapshot = configuration.clone();
    if let Some(fields) = snapshot.as_object_mut() {
        fields.remove("description");
        if let Some(widgets) = fields.get_mut("widgets").and_then(Value::as_array_mut) {
            for widget in widgets {
                let Some(settings) = widget.get_mut("settings").and_then(Value::as_object_mut)
                else {
                    continue;
                };
                if settings.get("kind").and_then(Value::as_str) == Some("Notes") {
                    settings.remove("text");
                    settings.remove("links");
                }
            }
        }
    }
    json!({ "name": name, "configuration": snapshot })
}

pub(super) fn deleted_configuration_snapshot(dashboard: &SavedDashboard) -> Option<Value> {
    // Deleting an unsupported definition remains possible without recording its unknown fields.
    let mut content = DashboardContent {
        name: dashboard.summary.name.clone(),
        configuration: serde_json::from_value(dashboard.configuration.clone()).ok()?,
    };
    content.validate().ok()?;
    Some(configuration_snapshot(
        &dashboard.summary.name,
        &dashboard.configuration,
    ))
}

pub(super) fn changes(
    renamed: bool,
    before: &DashboardConfiguration,
    after: &DashboardConfiguration,
) -> Vec<String> {
    let mut changes = Vec::new();
    for (changed, label) in [
        (renamed, "Name"),
        (before.description != after.description, "Description"),
        (before.tags != after.tags, "Tag filter"),
        (before.range_minutes != after.range_minutes, "Chart range"),
    ] {
        if changed {
            changes.push(label.into());
        }
    }
    let layout = before
        .widgets
        .iter()
        .map(|w| (w.id, w.layout))
        .ne(after.widgets.iter().map(|w| (w.id, w.layout)));
    if layout {
        changes.push("Layout".into());
    }
    if before.widgets.len() != after.widgets.len()
        || after.widgets.iter().any(|widget| {
            !before.widgets.iter().any(|old| {
                old.id == widget.id && old.title == widget.title && old.settings == widget.settings
            })
        })
    {
        changes.push("Widgets".into());
    }
    changes
}
