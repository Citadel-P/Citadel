//! Transport-neutral read results and supporting filters.
//! Repository implementations populate these types; server adapters map them to HTTP views.

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SwarmServiceDuplicateDraft {
    pub name: String,
    pub source_name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub tag_ids: Vec<Uuid>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SwarmServiceFilter {
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}
