//! Server-owned OpenAPI descriptions of discovery values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use uuid::Uuid;

schema_model! {
    citadel_discovery::lookup::LookupResourceInfo =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = LookupResourceInfo)]
    pub struct LookupResourceInfoSchema {
        pub id: Uuid,
        pub name: String,
        pub group: Option<String>,
    }
}
