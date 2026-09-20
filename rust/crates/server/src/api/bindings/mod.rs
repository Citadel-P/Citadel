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
pub struct BindingsHttpState {
    pub identity: Arc<IdentityService>,
    pub secrets: Arc<citadel_bindings::SecretService>,
    pub realtime: Option<RealtimeHub>,
}
pub(crate) fn metadata_error(error: citadel_bindings::BindingError) -> IdentityError {
    match error {
        citadel_bindings::BindingError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_bindings::BindingError::NotFound => IdentityError::NotFound,
        citadel_bindings::BindingError::Conflict(message) => IdentityError::Conflict(message),
        citadel_bindings::BindingError::Credential => IdentityError::Credential,
        citadel_bindings::BindingError::Storage(message) => IdentityError::Storage(message),
    }
}
