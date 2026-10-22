use crate::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRepositoryConfiguration {
    pub name: String,
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

/// References to stored S3 credentials; never contains secret values.
#[derive(Debug, Clone)]
pub struct BackupRepositoryCredentials {
    pub access_key_secret_id: Uuid,
    pub secret_key_secret_id: Uuid,
    pub session_token_secret_id: Option<Uuid>,
}

impl BackupRepositoryCredentials {
    pub fn apply(&self, spec: &mut crate::spec::BackupRepositorySpec) -> Result<(), BackupError> {
        if self.access_key_secret_id.is_nil()
            || self.secret_key_secret_id.is_nil()
            || self.session_token_secret_id.is_some_and(|id| id.is_nil())
        {
            return Err(BackupError::Validation(
                "Select valid stored S3 credential secrets.".into(),
            ));
        }
        match spec {
            crate::spec::BackupRepositorySpec::S3Compatible {
                access_key_secret_id,
                secret_key_secret_id,
                session_token_secret_id,
                ..
            } => {
                *access_key_secret_id = self.access_key_secret_id;
                *secret_key_secret_id = self.secret_key_secret_id;
                *session_token_secret_id = self.session_token_secret_id;
                Ok(())
            }
            _ => Err(BackupError::Validation(
                "Credential updates require an S3-compatible repository.".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_replacement_changes_only_s3_secret_references() {
        let old = Uuid::now_v7();
        let new = Uuid::now_v7();
        let mut spec: crate::spec::BackupRepositorySpec = serde_json::from_value(serde_json::json!({
            "$type":"S3Compatible", "endpoint":"https://s3.example.test", "bucket":"backups",
            "prefix":"prod", "accessKeySecretId":old, "secretKeySecretId":old, "sessionTokenSecretId":old
        })).unwrap();
        let credentials = BackupRepositoryCredentials {
            access_key_secret_id: new,
            secret_key_secret_id: new,
            session_token_secret_id: None,
        };
        credentials.apply(&mut spec).unwrap();
        let changed = serde_json::to_value(&spec).unwrap();
        assert_eq!(changed["endpoint"], "https://s3.example.test");
        assert_eq!(changed["bucket"], "backups");
        assert_eq!(changed["prefix"], "prod");
        assert_eq!(changed["accessKeySecretId"], serde_json::json!(new));
        assert!(changed["sessionTokenSecretId"].is_null());
        let invalid = BackupRepositoryCredentials {
            access_key_secret_id: Uuid::nil(),
            ..credentials.clone()
        };
        assert!(invalid.apply(&mut spec).is_err());
        let mut filesystem = crate::spec::BackupRepositorySpec::FileSystem {
            location: crate::spec::BackupExecutionLocation::Core,
            platform_id: None,
            path: "/backups".into(),
        };
        assert!(credentials.apply(&mut filesystem).is_err());
    }

    #[test]
    fn s3_endpoint_whitespace_is_removed_without_bypassing_transport_validation() {
        let mut input = BackupRepositoryConfiguration {
            name: "S3 test".into(),
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
