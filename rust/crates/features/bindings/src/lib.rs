#![forbid(unsafe_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod model;
pub use model::{
    BindingValidation, ResourceBinding, ResourceBindingKind, ResourceBindingScope,
    ResourceBindings, SecretDefinition, SecretDeliveryMode, SecretProvider, SecretProviderType,
};
mod commands;
pub use commands::{
    ExternalSecretInput, ExternalSecretPatch, NewResourceBinding, ResourceBindingInput,
    SecretProviderInput, SecretProviderPatch, StoredSecretProviderInput, StoredSecretProviderPatch,
};
mod validation;
pub use validation::{validate_binding, validate_provider};

pub(crate) use validation::validate_external_secret;

mod repository;
pub use repository::BindingRepository;
mod error;
use citadel_primitives::PatchField;
pub use error::BindingError;
mod name_validation;
use name_validation::validate_name;
pub use repository::SecretProtector;
mod service;
pub use service::SecretService;
pub mod secret_providers;
pub use secret_providers::{
    SecretProviderTester, SecretTestResult, TestExternalSecretInput, TestSecretProviderInput,
};
