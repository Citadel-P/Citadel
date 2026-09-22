use crate::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryConfiguration {
    pub name: String,
    pub description: Option<String>,
    pub spec: Value,
    pub password_secret_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s3_endpoint_whitespace_is_removed_without_bypassing_transport_validation() {
        let mut input = BackupRepositoryConfiguration {
            name: "S3 test".into(),
            description: None,
            password_secret_id: Uuid::now_v7(),
            spec: serde_json::json!({
                "$type": "S3Compatible",
                "endpoint": " \thttps://host.docker.internal:9000 \n",
                "bucket": "citadel-bucket",
                "accessKeySecretId": Uuid::now_v7(),
                "secretKeySecretId": Uuid::now_v7()
            }),
        };
        input.validate().unwrap();
        assert_eq!(input.spec["endpoint"], "https://host.docker.internal:9000");
        input.spec["endpoint"] = " http://host.docker.internal:9000 ".into();
        assert!(input.validate().is_err());
        input.spec["allowInsecureHttp"] = true.into();
        input.validate().unwrap();
        assert_eq!(input.spec["endpoint"], "http://host.docker.internal:9000");
        input.spec["endpoint"] = "https://bad host:9000".into();
        assert!(input.validate().is_err());
    }
}

impl BackupRepositoryConfiguration {
    pub fn validate(&mut self) -> Result<(), BackupError> {
        validate_name(&mut self.name, "Backup Repository")?;
        if self.password_secret_id.is_nil() {
            return Err(BackupError::Validation(
                "Backup Repository password Secret is required.".into(),
            ));
        }
        match discriminator(&self.spec)? {
            "FileSystem" => {
                let location = required_string(&self.spec, "location")?;
                let path = required_string(&self.spec, "path")?;
                let platform = self
                    .spec
                    .get("platformId")
                    .and_then(Value::as_str)
                    .and_then(|v| Uuid::parse_str(v).ok());
                if path.trim().is_empty()
                    || !matches!(location, "Core" | "Platform")
                    || (location == "Core" && platform.is_some())
                    || (location == "Platform" && platform.is_none())
                {
                    return Err(BackupError::Validation(
                        "Filesystem Backup Repository location is invalid.".into(),
                    ));
                }
            }
            "S3Compatible" => {
                if let Some(Value::String(endpoint)) = self.spec.get_mut("endpoint") {
                    *endpoint = endpoint.trim().to_owned();
                }
                let endpoint = required_string(&self.spec, "endpoint")?;
                let url = reqwest_url(endpoint)?;
                let insecure = self
                    .spec
                    .get("allowInsecureHttp")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if url.scheme() == "http" && !insecure {
                    return Err(BackupError::Validation(
                        "S3 endpoint must use HTTPS unless insecure HTTP is explicitly allowed."
                            .into(),
                    ));
                }
                if required_string(&self.spec, "bucket")?.trim().is_empty() {
                    return Err(BackupError::Validation("S3 bucket is required.".into()));
                }
                for key in ["accessKeySecretId", "secretKeySecretId"] {
                    required_uuid(&self.spec, key)?;
                }
            }
            _ => {
                return Err(BackupError::Validation(
                    "Backup Repository type is unsupported.".into(),
                ));
            }
        }
        Ok(())
    }
}
