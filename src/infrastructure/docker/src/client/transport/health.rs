//! Health-only reads retain the same transport limits as full inspection.
use super::{DockerClient, DockerError, MAX_JSON_BODY_BYTES, bounded_body};

impl DockerClient {
    pub(in crate::client) async fn health_inspection_bytes(
        &self,
        id: &str,
    ) -> Result<Vec<u8>, DockerError> {
        let version = self.negotiated_version().await?;
        let response = self
            .client
            .get(format!(
                "{}/v{version}/containers/{}/json",
                self.base_url,
                urlencoding::encode(id)
            ))
            .timeout(self.request_timeout)
            .send()
            .await?;
        let response = self.checked_response(response).await?;
        bounded_body(response, MAX_JSON_BODY_BYTES).await
    }
}
