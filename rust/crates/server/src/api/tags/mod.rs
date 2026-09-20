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
pub struct TagsHttpState {
    pub identity: Arc<IdentityService>,
    pub tags: Arc<dyn citadel_tags::TagRepository>,
    pub realtime: Option<RealtimeHub>,
}
pub(crate) fn metadata_error(error: citadel_tags::TagError) -> IdentityError {
    match error {
        citadel_tags::TagError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_tags::TagError::NotFound => IdentityError::NotFound,
        citadel_tags::TagError::Conflict(message) => IdentityError::Conflict(message),
        citadel_tags::TagError::Storage(message) => IdentityError::Storage(message),
    }
}
