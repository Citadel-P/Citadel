pub mod dto;
pub(crate) mod handlers;
use crate::api::resource_access::{
    authorize_global, authorize_resource, capabilities, publish_resource_change, require_actor,
};
use crate::realtime::RealtimeHub;
use crate::request_validation::invalid_json;
use citadel_identity::{IdentityError, IdentityService};
pub use handlers::router;
use std::sync::Arc;
#[derive(Clone)]
pub struct RegistriesHttpState {
    pub identity: Arc<IdentityService>,
    pub registries: Arc<dyn citadel_registries::RegistryRepository>,
    pub realtime: Option<RealtimeHub>,
}
pub(crate) fn metadata_error(error: citadel_registries::RegistryError) -> IdentityError {
    match error {
        citadel_registries::RegistryError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_registries::RegistryError::NotFound => IdentityError::NotFound,
        citadel_registries::RegistryError::Conflict(message) => IdentityError::Conflict(message),
        citadel_registries::RegistryError::Storage(message) => IdentityError::Storage(message),
    }
}
