use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewResourceBinding {
    pub name: String,
    pub kind: ResourceBindingKind,
    #[serde(skip)]
    pub scope: Option<ResourceBindingScope>,
    #[serde(skip)]
    pub resource_id: Option<Uuid>,
    pub value: Option<String>,
    pub secret_id: Option<Uuid>,
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingInput {
    pub id: Uuid,
    pub name: String,
    pub kind: ResourceBindingKind,
    pub value: Option<String>,
    pub secret_id: Option<Uuid>,
    pub secret_delivery_mode: Option<SecretDeliveryMode>,
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSecretInput {
    pub name: String,
    pub provider_id: Uuid,
    pub external_path: String,
    pub external_key: String,
    pub external_version: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSecretPatch {
    pub name: Option<String>,
    pub provider_id: Option<Uuid>,
    pub external_path: Option<String>,
    pub external_key: Option<String>,
    #[serde(default)]
    pub external_version: PatchField<i32>,
}

#[derive(Debug, Clone)]
pub struct StoredSecretProviderInput {
    pub name: String,
    pub address: String,
    pub mount_path: String,
    pub protected_token: String,
}

#[derive(Debug, Clone, Default)]
pub struct StoredSecretProviderPatch {
    pub name: Option<String>,
    pub address: Option<String>,
    pub mount_path: Option<String>,
    pub protected_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretProviderInput {
    pub name: String,
    pub address: String,
    pub mount_path: String,
    pub token: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretProviderPatch {
    pub name: Option<String>,
    pub address: Option<String>,
    pub mount_path: Option<String>,
    pub token: Option<String>,
}
