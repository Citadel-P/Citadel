use crate::{ManagedSwarmServiceView, SwarmServiceError, SwarmServiceSpec};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceAdoptionSource {
    pub docker_service_id: String,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceAdoptionIssue {
    pub code: String,
    pub message: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceAdoptionDraft {
    pub source: SwarmServiceAdoptionSource,
    pub name: String,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub issues: Vec<SwarmServiceAdoptionIssue>,
    pub preview_fingerprint: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdoptSwarmServiceInput {
    pub name: String,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub preview_fingerprint: String,
    #[serde(default, deserialize_with = "crate::model::deserialize_null_default")]
    pub tag_ids: Vec<Uuid>,
}
pub trait SwarmServiceAdoptionPort: Send + Sync {
    fn draft<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<SwarmServiceAdoptionDraft, SwarmServiceError>>;
    fn adopt<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        id: &'a str,
        input: &'a AdoptSwarmServiceInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ManagedSwarmServiceView, SwarmServiceError>>;
}
