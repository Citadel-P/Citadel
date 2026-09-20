use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebhookResponse {
    pub(crate) accepted: bool,
    pub(crate) status: &'static str,
    pub(crate) request_id: Uuid,
    pub(crate) reason: Option<&'static str>,
}
