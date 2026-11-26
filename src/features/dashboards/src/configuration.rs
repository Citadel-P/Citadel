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
    FleetSummary {
        measures: Vec<FleetMeasure>,
    },
    NeedsAttention {
        tab: AttentionTab,
        kinds: Vec<FleetKind>,
        limit: u8,
    },
    ResourceState {
        resource_kind: StateKind,
    },
    Watchlist {
        pins: Vec<MetricSource>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSource {
    pub kind: crate::SourceKind,
    pub id: Uuid,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum FleetMeasure {
    Platforms,
    Workloads,
    Alerts,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AttentionTab {
    All,
    Problems,
    Updates,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum FleetKind {
    Platform,
    Deployment,
    Stack,
    SwarmService,
    BackupPolicy,
    Build,
    BuildAgentPool,
    Registry,
    GitRepository,
    Script,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum StateKind {
    Platform,
    Deployment,
    Stack,
    SwarmService,
    Container,
}
