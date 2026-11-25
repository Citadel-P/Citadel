use uuid::Uuid;

#[derive(Clone, Copy, serde::Deserialize)]
pub enum SourceKind {
    Platform,
    Deployment,
    Stack,
    Container,
    SwarmService,
}

pub struct SourceFilter {
    pub kind: SourceKind,
    pub search: String,
    pub ids: Vec<Uuid>,
    pub tags: Vec<String>,
    pub limit: i64,
}

pub struct DashboardSource {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub platform_id: Option<Uuid>,
    pub container_projection_id: Option<Uuid>,
    pub docker_service_id: Option<String>,
    pub matches_tags: bool,
    pub exceeds_chart_series_limit: bool,
}

pub struct DashboardWorkload {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub status: String,
    pub processing: bool,
    pub probes: Vec<citadel_primitives::health::HealthStatus>,
    pub more_probes: bool,
    pub docker_id: Option<String>,
    pub node_id: Option<String>,
    pub stale: bool,
    pub can_view_logs: bool,
}

impl From<crate::WorkloadKind> for SourceKind {
    fn from(kind: crate::WorkloadKind) -> Self {
        match kind {
            crate::WorkloadKind::Container => Self::Container,
            crate::WorkloadKind::Deployment => Self::Deployment,
            crate::WorkloadKind::Stack => Self::Stack,
            crate::WorkloadKind::SwarmService => Self::SwarmService,
        }
    }
}
