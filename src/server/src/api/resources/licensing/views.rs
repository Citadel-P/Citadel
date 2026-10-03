use chrono::{DateTime, Utc};
use citadel_licensing::{LicenseCapability, LicenseState, LicenseStatus};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseCapabilityView {
    #[schema(value_type = crate::api::resources::vocabulary::LicenseCapabilitySchema)]
    pub capability: LicenseCapability,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseView {
    #[schema(value_type = crate::api::resources::vocabulary::LicenseStatusSchema)]
    pub status: LicenseStatus,
    pub effective_edition: String,
    #[schema(required = true)]
    pub licensed_edition: Option<String>,
    pub instance_id: Uuid,
    #[schema(required = true)]
    pub license_schema: Option<u8>,
    #[schema(required = true)]
    pub license_id: Option<String>,
    #[schema(required = true)]
    pub replaced_license_id: Option<String>,
    #[schema(required = true)]
    pub customer_id: Option<String>,
    #[schema(required = true)]
    pub customer_name: Option<String>,
    #[schema(required = true)]
    pub fingerprint: Option<String>,
    #[schema(required = true)]
    pub issued_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub not_before: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub expires_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub grace_until: Option<DateTime<Utc>>,
    pub capabilities: Vec<LicenseCapabilityView>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseEntitlementsView {
    #[schema(value_type = crate::api::resources::vocabulary::LicenseStatusSchema)]
    pub status: LicenseStatus,
    pub effective_edition: String,
    pub capabilities: Vec<LicenseCapabilityView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseRequestView {
    pub product: &'static str,
    pub instance_id: Uuid,
    pub core_version: String,
    pub generated_at: DateTime<Utc>,
}

fn capability_views(state: &LicenseState) -> Vec<LicenseCapabilityView> {
    LicenseCapability::ALL
        .iter()
        .copied()
        .map(|capability| LicenseCapabilityView {
            capability,
            enabled: state.capability_enabled(capability),
        })
        .collect()
}

pub fn to_view(state: &LicenseState) -> LicenseView {
    LicenseView {
        status: state.status,
        effective_edition: state.effective_edition.clone(),
        licensed_edition: state.licensed_edition.clone(),
        instance_id: state.instance_id,
        license_schema: state.license_schema,
        license_id: state.license_id.clone(),
        replaced_license_id: state.replaced_license_id.clone(),
        customer_id: state.customer_id.clone(),
        customer_name: state.customer_name.clone(),
        fingerprint: state.fingerprint.clone(),
        issued_at: state.issued_at,
        not_before: state.not_before,
        expires_at: state.expires_at,
        grace_until: state.grace_until,
        capabilities: capability_views(state),
        warnings: state.warnings.clone(),
    }
}

pub fn entitlements_view(state: &LicenseState) -> LicenseEntitlementsView {
    LicenseEntitlementsView {
        status: state.status,
        effective_edition: state.effective_edition.clone(),
        capabilities: capability_views(state),
    }
}

pub fn request_view(request: citadel_licensing::LicenseRequest) -> LicenseRequestView {
    LicenseRequestView {
        product: request.product,
        instance_id: request.instance_id,
        core_version: request.core_version,
        generated_at: request.generated_at,
    }
}
