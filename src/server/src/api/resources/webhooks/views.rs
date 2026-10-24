use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebhookResponse {
    pub(crate) accepted: bool,
    pub(crate) status: &'static str,
    pub(crate) request_id: Uuid,
    #[schema(required = true)]
    pub(crate) reason: Option<&'static str>,
}

/// Public webhook metadata. The credential is accepted only by mutation inputs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookView {
    pub enabled: bool,
    #[schema(value_type = crate::api::resources::schema_models::primitives::WebhookProviderSchema)]
    pub provider: citadel_primitives::WebhookProvider,
    #[schema(value_type = crate::api::resources::schema_models::primitives::WebhookAuthSchemeSchema)]
    pub auth_scheme: citadel_primitives::WebhookAuthScheme,
    pub has_secret: bool,
    pub branch_filter: Option<String>,
}
impl From<citadel_primitives::WebhookConfig> for WebhookView {
    fn from(value: citadel_primitives::WebhookConfig) -> Self {
        Self {
            enabled: value.enabled,
            provider: value.provider,
            auth_scheme: value.auth_scheme,
            has_secret: value
                .secret
                .as_deref()
                .is_some_and(|secret| !secret.trim().is_empty()),
            branch_filter: value.branch_filter,
        }
    }
}
pub fn serialize_optional<S: serde::Serializer>(
    value: &Option<citadel_primitives::WebhookConfig>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&value.clone().map(WebhookView::from), serializer)
}

#[cfg(test)]
mod security_tests {
    use super::*;
    #[test]
    fn read_views_never_serialize_saved_webhook_credentials() {
        let config: citadel_primitives::WebhookConfig =
            serde_json::from_value(serde_json::json!({"enabled":true,"secret":"sentinel-webhook"}))
                .unwrap();
        let value = serde_json::to_value(WebhookView::from(config)).unwrap();
        assert_eq!(value["hasSecret"], true);
        assert!(value.get("secret").is_none());
        let spec: citadel_stacks::StackSpec = serde_json::from_value(serde_json::json!({"$type":"Git","gitRepoId":uuid::Uuid::now_v7(),"branch":"main","webhook":{"enabled":true,"secret":"sentinel-stack"}})).unwrap();
        let value = serde_json::to_value(
            crate::api::resources::stacks::spec_view::StackSpecView::from(spec),
        )
        .unwrap();
        assert_eq!(value["webhook"]["hasSecret"], true);
        assert!(value["webhook"].get("secret").is_none());
        assert!(!value.to_string().contains("sentinel-stack"));
    }
}
