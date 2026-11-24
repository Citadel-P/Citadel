//! OpenAPI types mirror the serialized dashboard definition at the HTTP boundary.
use citadel_dashboards as domain;
use uuid::Uuid;

enum_schema!(
    DashboardVisibilitySchema,
    "DashboardVisibility",
    domain::DashboardVisibility,
    [Private, InstanceShared, Restricted]
);

enum_schema!(
    DashboardWorkloadKindSchema,
    "DashboardWorkloadKind",
    domain::WorkloadKind,
    [Container, Deployment, Stack, SwarmService]
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
    [Line, Area]
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

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "kind", deny_unknown_fields)]
#[schema(as = DashboardMetricSource)]
pub enum DashboardMetricSourceSchema {
    Platform {
        id: Uuid,
    },
    Container {
        id: Uuid,
        #[serde(rename = "platformId")]
        platform_id: Uuid,
    },
    Deployment {
        id: Uuid,
    },
    Stack {
        id: Uuid,
    },
    SwarmService {
        id: Uuid,
    },
}
impl From<domain::MetricSource> for DashboardMetricSourceSchema {
    fn from(value: domain::MetricSource) -> Self {
        match value {
            domain::MetricSource::Platform { id } => Self::Platform { id },
            domain::MetricSource::Container { id, platform_id } => {
                Self::Container { id, platform_id }
            }
            domain::MetricSource::Deployment { id } => Self::Deployment { id },
            domain::MetricSource::Stack { id } => Self::Stack { id },
            domain::MetricSource::SwarmService { id } => Self::SwarmService { id },
        }
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "kind", deny_unknown_fields)]
#[schema(as = DashboardWidgetSettings)]
pub enum DashboardWidgetSettingsSchema {
    ResourceSummary,
    Platforms {
        limit: u8,
    },
    Workloads {
        #[serde(rename = "resourceKind")]
        #[schema(value_type = crate::api::resources::schema_models::dashboards::DashboardWorkloadKindSchema)]
        resource_kind: domain::WorkloadKind,
        limit: u8,
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
            domain::WidgetSettings::ResourceSummary => Self::ResourceSummary,
            domain::WidgetSettings::Platforms { limit } => Self::Platforms { limit },
            domain::WidgetSettings::Workloads {
                resource_kind,
                limit,
            } => Self::Workloads {
                resource_kind,
                limit,
            },
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
