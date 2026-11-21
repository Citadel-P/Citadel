use base64::Engine;
use citadel_registries::RegistryConfiguration;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;
use zeroize::Zeroizing;

pub(crate) fn pull_credentials(
    id: Uuid,
    configuration: &Value,
) -> Result<Option<(&str, &str)>, &'static str> {
    let config = RegistryConfiguration(configuration);
    match config.kind() {
        "Custom" | "GitHub" | "DockerHub" => Ok(config.user_password()),
        "" if id == citadel_registries::DEFAULT_DOCKER_HUB_ID => Ok(None),
        _ => Err("This Registry type does not support image pull credentials."),
    }
}
pub(crate) fn registry_auth(
    id: Uuid,
    host: &str,
    configuration: &Value,
) -> Result<Option<Zeroizing<String>>, &'static str> {
    let Some((username, password)) = pull_credentials(id, configuration)? else {
        return Ok(None);
    };
    #[derive(Serialize)]
    struct Auth<'a> {
        username: &'a str,
        password: &'a str,
        serveraddress: &'a str,
    }
    let bytes = Zeroizing::new(
        serde_json::to_vec(&Auth {
            username,
            password,
            serveraddress: host,
        })
        .map_err(|_| "Invalid Registry credentials.")?,
    );
    Ok(Some(Zeroizing::new(
        base64::engine::general_purpose::STANDARD.encode(&*bytes),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pulls_allow_the_anonymous_default_and_only_supported_providers() {
        let id = Uuid::now_v7();
        let empty = json!({});
        assert!(
            registry_auth(
                citadel_registries::DEFAULT_DOCKER_HUB_ID,
                "docker.io",
                &empty
            )
            .unwrap()
            .is_none()
        );
        assert!(pull_credentials(id, &empty).is_err());
        let azure = json!({"$type":"Azure","userName":"some-user","password":"test-only"});
        assert!(RegistryConfiguration(&azure).user_password().is_some());
        assert!(pull_credentials(id, &azure).is_err());
    }
}
