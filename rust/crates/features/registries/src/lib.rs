#![forbid(unsafe_code)]
use citadel_tags::TagSummary;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
mod model;
pub use model::{RegistryDetails, RegistryStatus};
mod commands;
pub use commands::{NewRegistry, RegistryPatch};
mod validation;
pub use validation::{registry_type, validate_description, validate_name_identifier};

mod repository;
pub use repository::RegistryConnectionChecker;
pub use repository::RegistryRepository;
mod service;
pub use service::{create_registry, update_registry};
mod error;
pub use error::RegistryError;
pub use repository::RegistryMutationKind;
mod patch;
pub mod registry_images;
use citadel_primitives::PatchField;
