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

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretProviderInput {
    pub name: String,
    pub address: String,
    pub mount_path: String,
    pub token: String,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretProviderPatch {
    pub name: Option<String>,
    pub address: Option<String>,
    pub mount_path: Option<String>,
    pub token: Option<String>,
}

impl std::fmt::Debug for SecretProviderInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretProviderInput")
            .field("name", &self.name)
            .field("address", &self.address)
            .field("mount_path", &self.mount_path)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for SecretProviderPatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretProviderPatch")
            .field("name", &self.name)
            .field("address", &self.address)
            .field("mount_path", &self.mount_path)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod credential_debug_tests {
    use super::*;

    #[test]
    fn provider_commands_redact_plaintext_tokens() {
        let input: SecretProviderInput = serde_json::from_str(r#"{"name":"vault","address":"https://vault.test","mountPath":"kv","token":"SECRET_TOKEN"}"#).unwrap();
        let patch: SecretProviderPatch =
            serde_json::from_str(r#"{"token":"SECRET_TOKEN"}"#).unwrap();
        assert!(!format!("{input:?} {patch:?}").contains("SECRET_TOKEN"));
        assert_eq!(input.token, "SECRET_TOKEN");
        assert_eq!(patch.token.as_deref(), Some("SECRET_TOKEN"));
    }
}
