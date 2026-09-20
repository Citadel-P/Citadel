use crate::RuntimeCapabilityError;
use futures_util::{Stream, future::BoxFuture};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullImageInput {
    pub platform_id: Uuid,
    pub registry_id: Uuid,
    pub image_tag: String,
}
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullImageStreamItem {
    pub id: Option<String>,
    pub from: Option<String>,
    pub stream: Option<String>,
    pub status: Option<String>,
    pub error_message: Option<String>,
    pub progress_message: Option<String>,
    pub docker_image_id: Option<String>,
    pub digest: Option<String>,
    pub progress: Option<ImagePullProgress>,
    pub error: Option<ImagePullError>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePullProgress {
    pub units: Option<String>,
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub start: Option<i64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePullError {
    pub code: Option<i64>,
    pub message: Option<String>,
}
pub type ImagePullStream =
    Pin<Box<dyn Stream<Item = Result<PullImageStreamItem, RuntimeCapabilityError>> + Send>>;
pub trait ImagePullPort: Send + Sync {
    fn pull_image_stream<'a>(
        &'a self,
        image: &'a str,
        auth: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImagePullStream, RuntimeCapabilityError>>;
}
