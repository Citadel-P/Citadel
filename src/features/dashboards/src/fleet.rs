use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetReason {
    pub code: String,
    pub occurrences: i64,
    pub target: Option<Uuid>,
    pub observed_at: Option<String>,
    pub evidence: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetRow {
    pub kind: String,
    pub id: Uuid,
    pub platform_id: Option<Uuid>,
    pub name: String,
    pub state: String,
    pub reasons: Vec<FleetReason>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateCount {
    pub kind: String,
    pub state: String,
    pub count: i64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetPage {
    pub states: Vec<StateCount>,
    pub alerts: i64,
    pub total: i64,
    pub problems: i64,
    pub updates: i64,
    pub rows: Vec<FleetRow>,
}
#[derive(Clone, Copy, serde::Deserialize)]
pub enum FleetRead {
    Summary,
    Attention,
    OpenAlerts,
    States,
    Members,
}
pub struct FleetFilter {
    pub read: FleetRead,
    pub tags: Vec<String>,
    pub kinds: Vec<String>,
    pub state: Option<String>,
    pub tab: crate::AttentionTab,
    pub offset: i64,
    pub limit: i64,
}
