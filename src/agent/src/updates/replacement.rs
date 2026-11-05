use citadel_docker::client::DockerClient;
use citadel_platforms::containers::ContainerAction;
use std::time::Duration;

/// Runs in a separate container. Source and failed replacement are retained for inspection.
pub async fn replace(
    source_id: &str,
    source_image: &str,
    image: &str,
    operation: &str,
) -> Result<(), crate::app::AgentError> {
    uuid::Uuid::parse_str(operation)?;
    super::validate_image(image)?;
    let docker = DockerClient::new("/var/run/docker.sock", Duration::from_secs(30))?;
    tokio::time::timeout(Duration::from_secs(180), async {
        let source = docker.inspect_container_document(source_id).await?;
        if source["Id"] != source_id || source["Image"] != source_image {
            return Err("Agent source identity changed".into());
        }
        let suffix = format!("{operation}-{}", &source_id[..12.min(source_id.len())]);
        let configuration = super::replacement_config(&source, image, operation)?;
        let name = source["Name"]
            .as_str()
            .ok_or("Agent name is missing")?
            .trim_start_matches('/');
        let next = docker
            .create_container(&format!("citadel-agent-next-{suffix}"), &configuration)
            .await?;
        // Prevent the retained source from restarting beside its replacement after a daemon reboot.
        docker.disable_container_restart(source_id).await?;
        // The helper is independently owned before any stop. The old container and data remain.
        docker
            .change_container_state(source_id, ContainerAction::Stop)
            .await?;
        docker
            .rename_container(source_id, &format!("citadel-agent-previous-{suffix}"))
            .await?;
        docker.rename_container(&next, name).await?;
        docker.start_container(&next).await?;
        Ok::<_, crate::app::AgentError>(())
    })
    .await??;
    Ok(())
}
