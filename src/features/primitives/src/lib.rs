#![forbid(unsafe_code)]

pub mod actor;
pub use actor::ActorId;
pub mod permissions;
pub use permissions::{PermissionLevel, ResourceType, SpecificPermission};
pub mod authorization;
pub use authorization::{
    EffectivePermission, PermissionPolicy, PermissionRequirement, SpecificPermissions,
};
pub mod redaction;
pub use redaction::is_sensitive_environment_name;

pub mod patch;
pub use patch::{FieldUpdate, PatchField, merge_json};
pub mod control;
pub mod schedule;
pub use control::ResourceControlState;
pub mod audit;
pub use audit::AuditMetadata;

pub mod auto_update;
pub use auto_update::{AutoUpdateState, AutoUpdateStatus, UpdateBehavior, image_digests_equal};

pub mod webhook;
pub use webhook::{WebhookAuthScheme, WebhookConfig, WebhookPatch, WebhookProvider};

mod platform;
mod status;
pub use platform::PlatformStatus;

pub use authorization::AuthorizedResource;

pub mod json_keys;
pub mod normalization;
