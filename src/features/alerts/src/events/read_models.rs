use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct AlertEventFilter {
    pub resource_id: Option<Uuid>,
    pub alert_type: Option<String>,
    pub resource_type: Option<String>,
    pub unresolved_only: bool,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone)]
pub struct AlertEventPage {
    pub items: Vec<AlertEvent>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
}
