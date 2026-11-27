//! OpenAPI types mirror the serialized dashboard definition at the HTTP boundary.
use citadel_dashboards as domain;
use uuid::Uuid;

enum_schema!(
    DashboardSharingModeSchema,
    "DashboardSharingMode",
    domain::DashboardSharingMode,
    [InstanceShared, Restricted]
);

enum_schema!(
    DashboardVisibilitySchema,
    "DashboardVisibility",
    domain::DashboardVisibility,
    [Private, InstanceShared, Restricted]
);

enum_schema!(
    DashboardMetricSchema,
    "DashboardMetric",
    domain::Metric,
    [Cpu, Memory, Network, DiskIo]
);

enum_schema!(
    DashboardChartStyleSchema,
    "DashboardChartStyle",
    domain::ChartStyle,
    [Line, Area, Gradient]
);

enum_schema!(
    DashboardChartColorSchema,
    "DashboardChartColor",
    domain::ChartColor,
    [Blue, Green, Amber, Violet, Cyan]
);

schema_model! {
    domain::DashboardConfiguration =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = DashboardConfiguration)]
    pub struct DashboardConfigurationSchema {
        pub schema_version: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        pub tags: Vec<String>,
        pub range_minutes: u16,
        #[schema(value_type = Vec<crate::api::resources::schema_models::dashboards::DashboardWidgetSchema>)]
        pub widgets: Vec<domain::Widget>,
    }
}

schema_model! {
    domain::Widget =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = DashboardWidget)]
    pub struct DashboardWidgetSchema {
        pub id: Uuid,
        pub title: String,
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardLayoutSchema)]
        pub layout: domain::Layout,
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardWidgetSettingsSchema)]
        pub settings: domain::WidgetSettings,
    }
}

schema_model! {
    domain::Layout =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = DashboardLayout)]
    pub struct DashboardLayoutSchema {
        pub x: u16,
        pub y: u16,
        pub w: u16,
        pub h: u16,
    }
}

schema_model! {
    domain::NoteLink =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = DashboardNoteLink)]
    pub struct DashboardNoteLinkSchema {
        pub label: String,
        pub href: String,
    }
}

enum_schema!(
    DashboardSourceKindSchema,
    "DashboardSourceKind",
    domain::SourceKind,
    [Platform, Deployment, Stack, SwarmService]
);

schema_model! {
    domain::MetricSource =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(deny_unknown_fields)]
    #[schema(as = DashboardMetricSource)]
    pub struct DashboardMetricSourceSchema {
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardSourceKindSchema)]
        pub kind: domain::SourceKind,
        pub id: Uuid,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "kind", deny_unknown_fields)]
#[schema(as = DashboardWidgetSettings)]
pub enum DashboardWidgetSettingsSchema {
    FleetSummary {
        #[schema(value_type=Vec<crate::api::resources::schema_models::dashboards::DashboardFleetMeasureSchema>)]
        measures: Vec<domain::FleetMeasure>,
    },
    NeedsAttention {
        #[schema(value_type=crate::api::resources::schema_models::dashboards::DashboardAttentionTabSchema)]
        tab: domain::AttentionTab,
        #[schema(value_type=Vec<crate::api::resources::schema_models::dashboards::DashboardFleetKindSchema>)]
        kinds: Vec<domain::FleetKind>,
        limit: u8,
    },
    ResourceState {
        #[serde(rename = "resourceKind")]
        #[schema(value_type=crate::api::resources::schema_models::dashboards::DashboardStateKindSchema)]
        resource_kind: domain::StateKind,
    },
    Watchlist {
        #[schema(value_type=Vec<crate::api::resources::schema_models::dashboards::DashboardMetricSourceSchema>)]
        pins: Vec<domain::MetricSource>,
    },

    Metrics {
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardMetricSourceSchema)]
        source: domain::MetricSource,
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardMetricSchema)]
        metric: domain::Metric,
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardChartStyleSchema)]
        presentation: domain::ChartStyle,
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardChartColorSchema)]
        color: domain::ChartColor,
        legend: bool,
    },
    OpenAlerts {
        limit: u8,
    },
    Backups {
        limit: u8,
    },
    RecentActivity {
        limit: u8,
    },
    UpcomingSchedules {
        limit: u8,
    },
    Updates,
    Notes {
        text: String,
        #[schema(value_type = Vec<crate::api::resources::schema_models::dashboards::DashboardNoteLinkSchema>)]
        links: Vec<domain::NoteLink>,
    },
}
impl From<domain::WidgetSettings> for DashboardWidgetSettingsSchema {
    fn from(value: domain::WidgetSettings) -> Self {
        match value {
            domain::WidgetSettings::FleetSummary { measures } => Self::FleetSummary { measures },
            domain::WidgetSettings::NeedsAttention { tab, kinds, limit } => {
                Self::NeedsAttention { tab, kinds, limit }
            }
            domain::WidgetSettings::ResourceState { resource_kind } => {
                Self::ResourceState { resource_kind }
            }
            domain::WidgetSettings::Watchlist { pins } => Self::Watchlist { pins },
            domain::WidgetSettings::Metrics {
                source,
                metric,
                presentation,
                color,
                legend,
            } => Self::Metrics {
                source,
                metric,
                presentation,
                color,
                legend,
            },
            domain::WidgetSettings::OpenAlerts { limit } => Self::OpenAlerts { limit },
            domain::WidgetSettings::Backups { limit } => Self::Backups { limit },
            domain::WidgetSettings::RecentActivity { limit } => Self::RecentActivity { limit },
            domain::WidgetSettings::UpcomingSchedules { limit } => {
                Self::UpcomingSchedules { limit }
            }
            domain::WidgetSettings::Updates => Self::Updates,
            domain::WidgetSettings::Notes { text, links } => Self::Notes { text, links },
        }
    }
}
enum_schema!(
    DashboardFleetMeasureSchema,
    "DashboardFleetMeasure",
    domain::FleetMeasure,
    [Platforms, Workloads, Alerts]
);
enum_schema!(
    DashboardAttentionTabSchema,
    "DashboardAttentionTab",
    domain::AttentionTab,
    [All, Problems, Updates]
);
enum_schema!(
    DashboardFleetKindSchema,
    "DashboardFleetKind",
    domain::FleetKind,
    [
        Platform,
        Deployment,
        Stack,
        SwarmService,
        BackupPolicy,
        Build,
        BuildAgentPool,
        Registry,
        GitRepository,
        Script
    ]
);
enum_schema!(
    DashboardStateKindSchema,
    "DashboardStateKind",
    domain::StateKind,
    [Platform, Deployment, Stack, SwarmService, Container]
);
schema_model! { domain::FleetReason => #[derive(serde::Serialize,utoipa::ToSchema)] #[serde(rename_all="camelCase")] #[schema(as=DashboardFleetReason)] pub struct DashboardFleetReasonSchema {
pub code: String,
    pub occurrences:i64,
pub target: Option<Uuid>,
pub observed_at: Option<String>,
pub evidence: Option<String>,
}}
schema_model! { domain::FleetRow => #[derive(serde::Serialize,utoipa::ToSchema)] #[serde(rename_all="camelCase")] #[schema(as=DashboardFleetRow)] pub struct DashboardFleetRowSchema {
pub kind: String,
pub id: Uuid,
pub platform_id: Option<Uuid>,
pub name: String,
pub state: String,
#[schema(value_type=Vec<crate::api::resources::schema_models::dashboards::DashboardFleetReasonSchema>)]
pub reasons: Vec<domain::FleetReason>,
pub platform_name: Option<String>,
pub image: Option<String>,
pub created_at: Option<String>,
pub release_version: Option<String>,
pub published_ports: Option<Vec<String>>,
}}
schema_model! { domain::StateCount => #[derive(serde::Serialize,utoipa::ToSchema)] #[serde(rename_all="camelCase")] #[schema(as=DashboardStateCount)] pub struct DashboardStateCountSchema {
pub kind: String,
pub state: String,
pub count: i64,
}}
schema_model! { domain::FleetPage => #[derive(serde::Serialize,utoipa::ToSchema)] #[serde(rename_all="camelCase")] #[schema(as=DashboardFleetPage)] pub struct DashboardFleetPageSchema {
#[schema(value_type=Vec<crate::api::resources::schema_models::dashboards::DashboardStateCountSchema>)]
pub states: Vec<domain::StateCount>,
pub alerts: i64,
pub total: i64,
pub problems: i64,
pub updates: i64,
#[schema(value_type=Vec<crate::api::resources::schema_models::dashboards::DashboardFleetRowSchema>)]
pub rows: Vec<domain::FleetRow>,
}}
