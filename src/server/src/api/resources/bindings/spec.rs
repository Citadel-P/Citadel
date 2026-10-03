use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ResourceBindingKind {
    Variable,
    Secret,
}

impl From<ResourceBindingKind> for citadel_bindings::ResourceBindingKind {
    fn from(value: ResourceBindingKind) -> Self {
        match value {
            ResourceBindingKind::Variable => Self::Variable,
            ResourceBindingKind::Secret => Self::Secret,
        }
    }
}

impl From<citadel_bindings::ResourceBindingKind> for ResourceBindingKind {
    fn from(value: citadel_bindings::ResourceBindingKind) -> Self {
        match value {
            citadel_bindings::ResourceBindingKind::Variable => Self::Variable,
            citadel_bindings::ResourceBindingKind::Secret => Self::Secret,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ResourceBindingScope {
    Global,
    Stack,
    Deployment,
    SwarmService,
}

impl From<ResourceBindingScope> for citadel_bindings::ResourceBindingScope {
    fn from(value: ResourceBindingScope) -> Self {
        match value {
            ResourceBindingScope::Global => Self::Global,
            ResourceBindingScope::Stack => Self::Stack,
            ResourceBindingScope::Deployment => Self::Deployment,
            ResourceBindingScope::SwarmService => Self::SwarmService,
        }
    }
}

impl From<citadel_bindings::ResourceBindingScope> for ResourceBindingScope {
    fn from(value: citadel_bindings::ResourceBindingScope) -> Self {
        match value {
            citadel_bindings::ResourceBindingScope::Global => Self::Global,
            citadel_bindings::ResourceBindingScope::Stack => Self::Stack,
            citadel_bindings::ResourceBindingScope::Deployment => Self::Deployment,
            citadel_bindings::ResourceBindingScope::SwarmService => Self::SwarmService,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SecretDeliveryMode {
    EnvironmentVariable,
    MountedFile,
    NativePlatformSecret,
}

impl From<SecretDeliveryMode> for citadel_bindings::SecretDeliveryMode {
    fn from(value: SecretDeliveryMode) -> Self {
        match value {
            SecretDeliveryMode::EnvironmentVariable => Self::EnvironmentVariable,
            SecretDeliveryMode::MountedFile => Self::MountedFile,
            SecretDeliveryMode::NativePlatformSecret => Self::NativePlatformSecret,
        }
    }
}

impl From<citadel_bindings::SecretDeliveryMode> for SecretDeliveryMode {
    fn from(value: citadel_bindings::SecretDeliveryMode) -> Self {
        match value {
            citadel_bindings::SecretDeliveryMode::EnvironmentVariable => Self::EnvironmentVariable,
            citadel_bindings::SecretDeliveryMode::MountedFile => Self::MountedFile,
            citadel_bindings::SecretDeliveryMode::NativePlatformSecret => {
                Self::NativePlatformSecret
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SecretProviderType {
    InternalEncrypted,
    VaultCompatibleKvV2,
}

impl From<SecretProviderType> for citadel_bindings::SecretProviderType {
    fn from(value: SecretProviderType) -> Self {
        match value {
            SecretProviderType::InternalEncrypted => Self::InternalEncrypted,
            SecretProviderType::VaultCompatibleKvV2 => Self::VaultCompatibleKvV2,
        }
    }
}

impl From<citadel_bindings::SecretProviderType> for SecretProviderType {
    fn from(value: citadel_bindings::SecretProviderType) -> Self {
        match value {
            citadel_bindings::SecretProviderType::InternalEncrypted => Self::InternalEncrypted,
            citadel_bindings::SecretProviderType::VaultCompatibleKvV2 => Self::VaultCompatibleKvV2,
        }
    }
}
