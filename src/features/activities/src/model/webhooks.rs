use super::*;
/// Safe webhook audit metadata. Authentication headers, credentials and request
/// bodies are deliberately not representable in the persisted event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WebhookActivityDetails {
    pub request_id: Uuid,
    pub auth_type: String,
    pub execution: String,
    pub status: &'static str,
    pub reason: Option<&'static str>,
    #[serde(flatten)]
    pub source: WebhookActivitySource,
    pub dispatched_branch: Option<String>,
    pub dispatched_commit_sha: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WebhookActivitySource {
    pub delivery_id: Option<String>,
    pub event_type: Option<String>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub repository_full_name: Option<String>,
}
