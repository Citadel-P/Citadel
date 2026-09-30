use crate::SwarmServiceError;
use crate::SwarmServiceSpec;
use citadel_primitives::ActorId;
use citadel_primitives::AuthorizedResource;
use futures_util::future::BoxFuture;

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug)]
pub struct SwarmServiceAdoptionSource {
    pub docker_service_id: String,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
}
#[derive(Debug)]
pub struct SwarmServiceAdoptionIssue {
    pub code: String,
    pub message: String,
}
#[derive(Debug)]
pub struct SwarmServiceAdoptionDraft {
    pub source: SwarmServiceAdoptionSource,
    pub name: String,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub issues: Vec<SwarmServiceAdoptionIssue>,
    pub preview_fingerprint: String,
}
#[derive(Debug)]
pub struct AdoptSwarmService {
    pub name: String,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub preview_fingerprint: String,
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
        input: &'a AdoptSwarmService,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AuthorizedResource<crate::SwarmService>, SwarmServiceError>>;
}
