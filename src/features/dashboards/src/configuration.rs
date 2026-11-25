use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DashboardConfiguration {
    pub schema_version: u16,
    pub tags: Vec<String>,
    pub range_minutes: u16,
    pub widgets: Vec<Widget>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Widget {
    pub id: Uuid,
    pub title: String,
    pub layout: Layout,
    pub settings: WidgetSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum WidgetSettings {
    ResourceSummary,
    Platforms {
        limit: u8,
    },
    Workloads {
        resource_kind: WorkloadKind,
        limit: u8,
    },
    Metrics {
        source: MetricSource,
        metric: Metric,
        presentation: ChartStyle,
        color: ChartColor,
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
        links: Vec<NoteLink>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteLink {
    pub label: String,
    pub href: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkloadKind {
    Container,
    Deployment,
    Stack,
    SwarmService,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum MetricSource {
    Platform { id: Uuid },
    Container { id: Uuid, platform_id: Uuid },
    Deployment { id: Uuid },
    Stack { id: Uuid },
    SwarmService { id: Uuid },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Metric {
    Cpu,
    Memory,
    Network,
    DiskIo,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChartStyle {
    Line,
    Area,
    Gradient,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChartColor {
    Blue,
    Green,
    Amber,
    Violet,
    Cyan,
}
