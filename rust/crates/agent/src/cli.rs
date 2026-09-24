//! Local container liveness probe; independent of Core and Docker availability.
use crate::app::AgentError;
use crate::config::{AgentConfig, AgentMode};
use std::time::Duration;

pub async fn healthcheck() -> Result<(), AgentError> {
    let config = AgentConfig::from_environment()?;
    let tls = matches!(config.mode, AgentMode::Direct(ref direct) if direct.tls.is_some());
    let mut client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(2));
    if let AgentMode::Direct(direct) = &config.mode
        && let Some(tls) = &direct.tls
    {
        // Trust the installed chain for this loopback probe. Its public DNS name
        // need not include the loopback IP; outbound Core TLS is unchanged.
        let certificate = std::fs::read(&tls.certificate_path)?;
        client = client
            .tls_certs_only(reqwest::Certificate::from_pem_bundle(&certificate)?)
            .tls_danger_accept_invalid_hostnames(true);
    }
    let client = client.build()?;
    let scheme = if tls { "https" } else { "http" };
    let response = client
        .get(format!("{scheme}://127.0.0.1:{}/health", config.port))
        .send()
        .await?
        .error_for_status()?;
    if response.json::<serde_json::Value>().await?["Status"] != "Healthy" {
        return Err("Agent health response is not healthy".into());
    }
    Ok(())
}
