use super::*;

impl NewRegistry {
    pub fn validate(&mut self) -> Result<(), RegistryError> {
        validate_name_identifier(&self.name, "Registry")?;
        self.name = self.name.trim().to_owned();
        validate_description(self.description.as_deref())?;
        let kind = registry_type(&self.configuration)?;
        validate_registry_configuration(&self.configuration, &kind)?;
        if kind == "DockerHub" {
            self.registry_host = "docker.io".to_owned();
        } else if kind == "GitHub" {
            self.registry_host = "ghcr.io".to_owned();
        } else {
            self.registry_host = self.registry_host.trim().trim_end_matches('/').to_owned();
        }
        validate_registry_host(&self.registry_host)?;
        Ok(())
    }
}

pub fn validate_name_identifier(name: &str, resource: &str) -> Result<(), RegistryError> {
    let name = name.trim();
    if !(3..=64).contains(&name.len())
        || !name
            .bytes()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_'))
    {
        return Err(RegistryError::Validation(format!(
            "{resource} name must contain 3 to 64 letters, numbers, hyphens, or underscores."
        )));
    }
    Ok(())
}

fn validate_registry_host(host: &str) -> Result<(), RegistryError> {
    let (authority, path) = host
        .split_once('/')
        .map_or((host, None), |(host, path)| (host, Some(path)));
    let (hostname, port) = authority
        .rsplit_once(':')
        .filter(|(_, port)| port.bytes().all(|character| character.is_ascii_digit()))
        .map_or((authority, None), |(hostname, port)| (hostname, Some(port)));
    let valid_hostname = !hostname.is_empty()
        && hostname.len() <= 253
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|character| character.is_ascii_alphanumeric() || character == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        });
    let valid_port = port.is_none_or(|port| !port.is_empty() && port.len() <= 5);
    let valid_path = path.is_none_or(|path| {
        !path.is_empty()
            && path
                .bytes()
                .all(|character| character.is_ascii_alphanumeric() || b"._-/".contains(&character))
    });
    if !valid_hostname || !valid_port || !valid_path {
        return Err(RegistryError::Validation(
            "Registry host must be a valid host name such as ghcr.io.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_registry_configuration(configuration: &Value, kind: &str) -> Result<(), RegistryError> {
    let string_length = |name: &str, minimum: usize| {
        configuration_value(configuration, name)
            .and_then(Value::as_str)
            .is_some_and(|value| value.len() >= minimum)
    };
    let enabled = |name: &str| {
        configuration_value(configuration, name)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let valid = match kind {
        "DockerHub" => string_length("userName", 4) && string_length("pat", 10),
        "Azure" => string_length("userName", 3) && string_length("password", 6),
        "AWS" => {
            string_length("accessKey", 10)
                && string_length("secretAccessKey", 10)
                && string_length("region", 4)
        }
        "Gitlab" => {
            string_length("instanceUrl", 10)
                && string_length("userName", 10)
                && string_length("pat", 10)
        }
        "GitHub" => {
            !enabled("ghcrAuthEnabled")
                || (string_length("pat", 10) && string_length("nameSpace", 5))
        }
        "Custom" => {
            !enabled("authEnabled")
                || (string_length("userName", 3) && string_length("password", 6))
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(RegistryError::Validation(format!(
            "{kind} Registry credentials do not meet the required fields and minimum lengths."
        )))
    }
}

fn configuration_value<'a>(configuration: &'a Value, name: &str) -> Option<&'a Value> {
    configuration
        .as_object()?
        .iter()
        .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value))
}

pub fn registry_type(configuration: &Value) -> Result<String, RegistryError> {
    configuration
        .get("$type")
        .or_else(|| configuration.get("type"))
        .and_then(Value::as_str)
        .filter(|kind| {
            matches!(
                *kind,
                "Custom" | "DockerHub" | "Azure" | "AWS" | "Gitlab" | "GitHub"
            )
        })
        .map(str::to_owned)
        .ok_or_else(|| {
            RegistryError::Validation(
                "Registry configuration must include a supported type discriminator.".to_owned(),
            )
        })
}

pub fn validate_description(description: Option<&str>) -> Result<(), RegistryError> {
    if description.is_some_and(|value| value.chars().count() > 600) {
        return Err(RegistryError::Validation(
            "Description cannot exceed 600 characters.".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_validation_normalizes_known_hosts_and_rejects_untyped_credentials() {
        let mut docker_hub = NewRegistry {
            name: " Docker-Hub ".into(),
            registry_host: "ignored.example".into(),
            status: RegistryStatus::Active,
            configuration: serde_json::json!({
                "$type":"DockerHub",
                "userName":"user",
                "pat":"0123456789"
            }),
            description: None,
            tag_ids: Vec::new(),
        };
        docker_hub.validate().unwrap();
        assert_eq!(docker_hub.name, "Docker-Hub");
        assert_eq!(docker_hub.registry_host, "docker.io");

        docker_hub.configuration = serde_json::json!({"Username":"user"});
        assert!(docker_hub.validate().is_err());
    }
}
