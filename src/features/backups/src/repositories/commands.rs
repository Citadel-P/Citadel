use crate::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryConfiguration {
    pub name: String,
    pub description: Option<String>,
    pub spec: crate::spec::BackupRepositorySpec,
    pub password_secret_id: Uuid,
}

impl BackupRepositoryConfiguration {
    pub fn validate(&mut self) -> Result<(), BackupError> {
        validate_name(&mut self.name, "Backup Repository")?;
        if self.password_secret_id.is_nil() {
            return Err(BackupError::Validation(
                "Backup Repository password Secret is required.".into(),
            ));
        }
        self.spec.normalize();
        self.spec.validate()?;
        Ok(())
    }
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
            spec: serde_json::from_value(serde_json::json!({
                "$type": "S3Compatible",
                "endpoint": " \thttps://host.docker.internal:9000 \n",
                "bucket": "citadel-bucket",
                "accessKeySecretId": Uuid::now_v7(),
                "secretKeySecretId": Uuid::now_v7()
            }))
            .unwrap(),
        };
        input.validate().unwrap();
        assert_eq!(
            serde_json::to_value(&input.spec).unwrap()["endpoint"],
            "https://host.docker.internal:9000"
        );
        if let crate::spec::BackupRepositorySpec::S3Compatible { endpoint, .. } = &mut input.spec {
            *endpoint = " http://host.docker.internal:9000 ".into();
        }
        assert!(input.validate().is_err());
        if let crate::spec::BackupRepositorySpec::S3Compatible {
            allow_insecure_http,
            ..
        } = &mut input.spec
        {
            *allow_insecure_http = true;
        }
        input.validate().unwrap();
        assert_eq!(
            serde_json::to_value(&input.spec).unwrap()["endpoint"],
            "http://host.docker.internal:9000"
        );
        if let crate::spec::BackupRepositorySpec::S3Compatible { endpoint, .. } = &mut input.spec {
            *endpoint = "https://bad host:9000".into();
        }
        assert!(input.validate().is_err());
    }
}
