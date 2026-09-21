use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountDetails {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub actor_id: ActorId,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: ActorId,
    pub updated_at: DateTime<Utc>,
    pub archived_at_utc: Option<DateTime<Utc>>,
    pub active_token_count: i64,
    pub last_used_at_utc: Option<DateTime<Utc>>,
    pub teams: Vec<ResourceInfo>,
    pub roles: Vec<ResourceInfo>,
    pub resource_accesses: Vec<ServiceAccountResourceAccess>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAsActorUsageDetails {
    pub id: Uuid,
    pub name: String,
    pub resource_type: ResourceType,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountTokenDetails {
    pub id: Uuid,
    pub name: String,
    pub hint: String,
    pub expires_at_utc: Option<DateTime<Utc>>,
    pub last_used_at_utc: Option<DateTime<Utc>>,
    pub revoked_at_utc: Option<DateTime<Utc>>,
    pub revoked_by_actor_id: Option<ActorId>,
    pub created_by_actor_id: ActorId,
    pub created_by_name: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedServiceAccountTokenDetails {
    #[serde(flatten)]
    pub credential: ServiceAccountTokenDetails,
    pub token: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccountLimitsDetails {
    pub default_token_lifetime_days: i64,
    pub maximum_token_lifetime_days: i64,
    pub maximum_active_tokens_per_account: i64,
}

impl Default for ServiceAccountLimitsDetails {
    fn default() -> Self {
        Self {
            default_token_lifetime_days: super::DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
            maximum_token_lifetime_days: super::MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
            maximum_active_tokens_per_account: super::MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS,
        }
    }
}
