use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateSwarmService {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: Option<SwarmServiceDuplicateSource>,
}

#[derive(Debug, Clone)]
pub struct SwarmServiceDuplicateSource {
    pub resource_id: Uuid,
    pub resource_type: String,
    pub resource_name: String,
}

#[derive(Debug, Clone)]
pub struct UpdateSwarmService {
    pub spec: SwarmServiceSpec,
    pub row_version: i64,
}

#[derive(Debug, Clone)]
pub struct RenameSwarmService {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ScaleSwarmService {
    pub replicas: i32,
}
