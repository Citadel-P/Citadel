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
