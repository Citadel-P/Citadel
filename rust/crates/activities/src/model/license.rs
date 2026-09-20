use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LicenseActivitySnapshot {
    #[serde(rename = "Schema")]
    pub schema: Option<u8>,
    #[serde(rename = "LicenseId")]
    pub license_id: Option<String>,
    #[serde(rename = "ReplacedLicenseId")]
    pub replaced_license_id: Option<String>,
    #[serde(rename = "LicensedEdition")]
    pub licensed_edition: Option<String>,
    #[serde(rename = "EffectiveEdition")]
    pub effective_edition: String,
    #[serde(rename = "EffectiveCapabilities")]
    pub effective_capabilities: Vec<LicenseCapability>,
    #[serde(rename = "CustomerId")]
    pub customer_id: Option<String>,
    #[serde(rename = "CustomerName")]
    pub customer_name: Option<String>,
    #[serde(rename = "Fingerprint")]
    pub fingerprint: Option<String>,
    #[serde(rename = "Status")]
    pub status: LicenseStatus,
    #[serde(rename = "ExpiresAt")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(rename = "GraceUntil")]
    pub grace_until: Option<DateTime<Utc>>,
}
