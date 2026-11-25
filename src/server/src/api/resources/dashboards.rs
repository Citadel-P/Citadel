use super::schema_models::dashboards::{DashboardConfigurationSchema, DashboardVisibilitySchema};
use citadel_dashboards::{DashboardContent, DashboardVisibility};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDashboard {
    pub id: Uuid,
    #[schema(value_type = DashboardVisibilitySchema)]
    pub visibility: DashboardVisibility,
    pub name: String,
    #[schema(value_type = DashboardConfigurationSchema)]
    pub configuration: citadel_dashboards::DashboardConfiguration,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateDashboard {
    pub revision: i64,
    pub name: String,
    #[schema(value_type = DashboardConfigurationSchema)]
    pub configuration: citadel_dashboards::DashboardConfiguration,
}

impl From<UpdateDashboard> for DashboardContent {
    fn from(value: UpdateDashboard) -> Self {
        Self {
            name: value.name,
            configuration: value.configuration,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummaryView {
    pub id: Uuid,
    pub name: String,
    #[schema(value_type = DashboardVisibilitySchema)]
    pub visibility: DashboardVisibility,
    pub revision: i64,
    pub can_edit: bool,
    pub can_delete: bool,
}

impl From<citadel_dashboards::DashboardSummary> for DashboardSummaryView {
    fn from(value: citadel_dashboards::DashboardSummary) -> Self {
        let citadel_dashboards::DashboardSummary {
            id,
            name,
            visibility,
            revision,
            can_edit,
            can_delete,
        } = value;
        Self {
            id,
            name,
            visibility,
            revision,
            can_edit,
            can_delete,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedDashboardView {
    pub summary: DashboardSummaryView,
    /// Preserve unknown stored versions for read-only clients.
    pub configuration: serde_json::Value,
}

impl From<citadel_dashboards::SavedDashboard> for SavedDashboardView {
    fn from(value: citadel_dashboards::SavedDashboard) -> Self {
        Self {
            summary: value.summary.into(),
            configuration: value.configuration,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct DashboardList {
    pub dashboards: Vec<DashboardSummaryView>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DashboardDefault {
    pub dashboard_id: Option<Uuid>,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DashboardListQuery {
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DashboardRevision {
    pub revision: i64,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetDashboardVisibility {
    pub revision: i64,
    #[schema(value_type = super::schema_models::dashboards::DashboardSharingModeSchema)]
    pub visibility: citadel_dashboards::DashboardSharingMode,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DashboardSourceQuery {
    #[param(value_type=String)]
    pub kind: citadel_dashboards::SourceKind,
    pub search: Option<String>,
    /// Comma-separated pinned resource IDs of the selected kind.
    pub ids: Option<String>,
    pub limit: Option<u8>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSourceView {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub platform_id: Option<Uuid>,
    pub container_projection_id: Option<Uuid>,
    pub docker_service_id: Option<String>,
    pub matches_tags: bool,
    pub exceeds_chart_series_limit: bool,
}

impl From<citadel_dashboards::DashboardSource> for DashboardSourceView {
    fn from(row: citadel_dashboards::DashboardSource) -> Self {
        Self {
            id: row.id,
            name: row.name,
            status: row.status,
            platform_id: row.platform_id,
            container_projection_id: row.container_projection_id,
            docker_service_id: row.docker_service_id,
            matches_tags: row.matches_tags,
            exceeds_chart_series_limit: row.exceeds_chart_series_limit,
        }
    }
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DashboardWorkloadQuery {
    #[param(value_type=super::schema_models::dashboards::DashboardWorkloadKindSchema)]
    pub kind: citadel_dashboards::WorkloadKind,
    pub limit: Option<u8>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardWorkloadView {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub status: String,
    pub processing: bool,
    #[schema(value_type=Vec<super::platforms::workload_health_views::HealthStatus>)]
    pub observed_probes: Vec<citadel_primitives::health::HealthStatus>,
    pub more_probes: bool,
    #[schema(value_type=Option<super::schema_models::backups::BackupCoverageStatusSchema>)]
    pub backup_status: Option<citadel_backups::BackupCoverageStatus>,
    pub can_view_logs: bool,
}
