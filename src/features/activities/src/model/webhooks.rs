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

impl WebhookActivityDetails {
    /// Classify the delivery outcome consistently for every webhook target.
    /// Unknown skips are warnings so new blocking conditions are not hidden.
    pub fn activity_status(&self) -> ActivityStatus {
        match (self.status, self.reason) {
            ("queued", _) | ("noop", Some("Stack update notification queued.")) => {
                ActivityStatus::Success
            }
            (
                "noop",
                Some(
                    "Branch filter did not match"
                    | "Branch mismatch"
                    | "Unsupported event type"
                    | "No relevant path changes"
                    | "No new commit"
                    | "Build Project is disabled."
                    | "Stack is pinned to a commit."
                    | "Stack Git updates are disabled."
                    | "Service image updates are disabled."
                    | "Service image is up to date.",
                ),
            ) => ActivityStatus::Information,
            ("noop", _) => ActivityStatus::Warning,
            _ => ActivityStatus::Failure,
        }
    }
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
