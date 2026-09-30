//! Server-owned OpenAPI descriptions of bindings values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

schema_model! {
    citadel_bindings::secret_providers::SecretTestResult =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SecretTestResult)]
    pub struct SecretTestResultSchema {
        pub success: bool,
        pub message: String,
    }
}
