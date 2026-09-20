use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceBindingKind {
    Variable,
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceBindingScope {
    Global,
    Stack,
    Deployment,
    SwarmService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretDeliveryMode {
    EnvironmentVariable,
    MountedFile,
    NativePlatformSecret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretProviderType {
    InternalEncrypted,
    VaultCompatibleKvV2,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinding {
    pub id: Uuid,
    pub name: String,
    pub kind: ResourceBindingKind,
    pub scope: ResourceBindingScope,
    pub resource_id: Option<Uuid>,
    pub value: Option<String>,
    pub secret_id: Option<Uuid>,
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    pub target_path: Option<String>,
    pub is_inherited: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindings {
    pub entries: Vec<ResourceBinding>,
    pub effective_entries: Vec<ResourceBinding>,
}

#[derive(Clone, Copy)]
pub struct BindingValidation<'a> {
    pub name: &'a str,
    pub kind: ResourceBindingKind,
    pub scope: ResourceBindingScope,
    pub resource_id: Option<Uuid>,
    pub value: Option<&'a str>,
    pub secret_id: Option<Uuid>,
    pub delivery: Option<SecretDeliveryMode>,
    pub target_path: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDefinition {
    pub id: Uuid,
    pub name: String,
    pub provider_type: SecretProviderType,
    pub provider_id: Option<Uuid>,
    pub external_path: Option<String>,
    pub external_key: Option<String>,
    pub external_version: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretProvider {
    pub id: Uuid,
    pub name: String,
    pub provider_type: SecretProviderType,
    pub address: String,
    pub mount_path: String,
    pub created_at: DateTime<Utc>,
}
