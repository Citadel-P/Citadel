use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum BackupExecutionLocation {
    #[default]
    Core,
    Platform,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum S3BucketLookup {
    #[default]
    Auto,
    Path,
    Dns,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum VolumeBackupConsistency {
    #[default]
    Live,
    StopAttachedContainers,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "$type")]
pub enum BackupRepositorySpec {
    #[serde(rename_all = "camelCase")]
    FileSystem {
        location: BackupExecutionLocation,
        platform_id: Option<Uuid>,
        path: String,
    },
    #[serde(rename_all = "camelCase")]
    S3Compatible {
        endpoint: String,
        bucket: String,
        prefix: Option<String>,
        region: Option<String>,
        #[serde(default)]
        bucket_lookup: S3BucketLookup,
        access_key_secret_id: Uuid,
        secret_key_secret_id: Uuid,
        session_token_secret_id: Option<Uuid>,
        #[serde(default)]
        allow_insecure_http: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "$type")]
pub enum BackupSourceSpec {
    #[serde(rename_all = "camelCase")]
    DockerVolume {
        platform_id: Uuid,
        volume_name: String,
        docker_node_id: Option<String>,
        #[serde(default)]
        consistency: VolumeBackupConsistency,
    },
    CitadelSystem {},
    #[serde(rename_all = "camelCase")]
    Stack {
        stack_id: Uuid,
    },
    #[serde(rename_all = "camelCase")]
    Deployment {
        deployment_id: Uuid,
    },
    #[serde(rename_all = "camelCase")]
    SwarmService {
        swarm_service_id: Uuid,
    },
}

impl BackupRepositorySpec {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::FileSystem { .. } => "FileSystem",
            Self::S3Compatible { .. } => "S3Compatible",
        }
    }

    pub fn platform_id(&self) -> Option<Uuid> {
        match self {
            Self::FileSystem { platform_id, .. } => *platform_id,
            Self::S3Compatible { .. } => None,
        }
    }

    pub fn normalize(&mut self) {
        match self {
            Self::FileSystem { path, .. } => *path = path.trim().to_owned(),
            Self::S3Compatible {
                endpoint,
                bucket,
                prefix,
                region,
                ..
            } => {
                *endpoint = endpoint.trim().to_owned();
                *bucket = bucket.trim().to_owned();
                *prefix = prefix
                    .take()
                    .map(|v| v.trim().trim_matches('/').to_owned())
                    .filter(|v| !v.is_empty());
                *region = citadel_primitives::normalization::optional_text(region.take());
            }
        }
    }

    pub fn validate(&self) -> Result<(), crate::BackupError> {
        use crate::BackupError;
        match self {
            Self::FileSystem {
                location,
                platform_id,
                path,
            } => {
                let valid_location = match location {
                    BackupExecutionLocation::Core => platform_id.is_none(),
                    BackupExecutionLocation::Platform => platform_id.is_some_and(|id| !id.is_nil()),
                };
                if !valid_location || path.trim().is_empty() {
                    return Err(BackupError::Validation(
                        "Filesystem Backup Repository location is invalid.".into(),
                    ));
                }
            }
            Self::S3Compatible {
                endpoint,
                bucket,
                access_key_secret_id,
                secret_key_secret_id,
                session_token_secret_id,
                allow_insecure_http,
                ..
            } => {
                let url = crate::validation::reqwest_url(endpoint)?;
                if url.scheme() == "http" && !allow_insecure_http {
                    return Err(BackupError::Validation(
                        "S3 endpoint must use HTTPS unless insecure HTTP is explicitly allowed."
                            .into(),
                    ));
                }
                if bucket.trim().is_empty() {
                    return Err(BackupError::Validation("S3 bucket is required.".into()));
                }
                for (id, field) in [
                    (*access_key_secret_id, "accessKeySecretId"),
                    (*secret_key_secret_id, "secretKeySecretId"),
                ] {
                    valid_id(id, field)?;
                }
                if let Some(id) = session_token_secret_id {
                    valid_id(*id, "sessionTokenSecretId")?;
                }
            }
        }
        Ok(())
    }
}

impl BackupSourceSpec {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::DockerVolume { .. } => "DockerVolume",
            Self::CitadelSystem {} => "CitadelSystem",
            Self::Stack { .. } => "Stack",
            Self::Deployment { .. } => "Deployment",
            Self::SwarmService { .. } => "SwarmService",
        }
    }

    pub fn validate(&self) -> Result<(), crate::BackupError> {
        match self {
            Self::DockerVolume {
                platform_id,
                volume_name,
                ..
            } => {
                valid_id(*platform_id, "platformId")?;
                if volume_name.trim().is_empty() {
                    return Err(crate::BackupError::Validation(
                        "Backup field 'volumeName' is required.".into(),
                    ));
                }
            }
            Self::CitadelSystem {} => {}
            Self::Stack { stack_id } => valid_id(*stack_id, "stackId")?,
            Self::Deployment { deployment_id } => valid_id(*deployment_id, "deploymentId")?,
            Self::SwarmService { swarm_service_id } => {
                valid_id(*swarm_service_id, "swarmServiceId")?
            }
        }
        Ok(())
    }

    pub fn resource(&self) -> Option<(citadel_primitives::ResourceType, Uuid)> {
        use citadel_primitives::ResourceType;
        match self {
            Self::DockerVolume { platform_id, .. } => Some((ResourceType::Platform, *platform_id)),
            Self::Stack { stack_id } => Some((ResourceType::Stack, *stack_id)),
            Self::Deployment { deployment_id } => Some((ResourceType::Deployment, *deployment_id)),
            Self::SwarmService { swarm_service_id } => {
                Some((ResourceType::SwarmService, *swarm_service_id))
            }
            Self::CitadelSystem {} => None,
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::DockerVolume {
                platform_id,
                volume_name,
                docker_node_id,
                ..
            } => docker_volume_source_key(*platform_id, docker_node_id.as_deref(), volume_name),
            Self::CitadelSystem {} => "citadel-system".into(),
            Self::Stack { stack_id } => format!("stack:{stack_id}"),
            Self::Deployment { deployment_id } => format!("deployment:{deployment_id}"),
            Self::SwarmService { swarm_service_id } => format!("swarm-service:{swarm_service_id}"),
        }
    }
}

fn valid_id(id: Uuid, field: &str) -> Result<(), crate::BackupError> {
    if id.is_nil() {
        Err(crate::BackupError::Validation(format!(
            "Backup field '{field}' must not be an empty UUID."
        )))
    } else {
        Ok(())
    }
}

/// Stable identity shared by policy snapshots and per-volume execution leases.
pub fn docker_volume_source_key(platform: Uuid, node: Option<&str>, volume: &str) -> String {
    let volume = volume.trim();
    match node.map(str::trim).filter(|v| !v.is_empty()) {
        Some(node) => format!("{platform}:{node}:{volume}"),
        None => format!("{platform}:{volume}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn malformed_variant_fields_fail_at_deserialization() {
        for value in [
            json!({"$type":"FileSystem","location":"Unexpected","path":"/backups"}),
            json!({"$type":"FileSystem","location":"Core","platformId":"bad-id","path":"/backups"}),
            json!({"$type":"S3Compatible","endpoint":"https://s3.test","bucket":"backups","accessKeySecretId":Uuid::now_v7(),"secretKeySecretId":Uuid::now_v7(),"allowInsecureHttp":"true"}),
        ] {
            assert!(serde_json::from_value::<BackupRepositorySpec>(value).is_err());
        }
        for value in [
            json!({}),
            json!({"$type":"Unknown"}),
            json!({"$type":"Stack","stackId":"invalid"}),
            json!({"$type":"DockerVolume","platformId":Uuid::now_v7(),"volumeName":"data","consistency":"Unexpected"}),
        ] {
            assert!(serde_json::from_value::<BackupSourceSpec>(value).is_err());
        }
    }

    #[test]
    fn filesystem_location_requires_a_nonempty_platform_only_for_platform_execution() {
        for (location, platform_id, valid) in [
            (BackupExecutionLocation::Core, None, true),
            (BackupExecutionLocation::Core, Some(Uuid::now_v7()), false),
            (BackupExecutionLocation::Platform, None, false),
            (BackupExecutionLocation::Platform, Some(Uuid::nil()), false),
            (
                BackupExecutionLocation::Platform,
                Some(Uuid::now_v7()),
                true,
            ),
        ] {
            let spec = BackupRepositorySpec::FileSystem {
                location,
                platform_id,
                path: "/backups".into(),
            };
            assert_eq!(spec.validate().is_ok(), valid);
        }
    }

    #[test]
    fn s3_secret_references_reject_empty_uuids() {
        let id = Uuid::now_v7();
        for (access, secret, session, valid) in [
            (id, id, None, true),
            (id, id, Some(id), true),
            (Uuid::nil(), id, None, false),
            (id, Uuid::nil(), None, false),
            (id, id, Some(Uuid::nil()), false),
        ] {
            let spec = BackupRepositorySpec::S3Compatible {
                endpoint: "https://s3.example.test".into(),
                bucket: "backups".into(),
                prefix: None,
                region: None,
                bucket_lookup: S3BucketLookup::Auto,
                access_key_secret_id: access,
                secret_key_secret_id: secret,
                session_token_secret_id: session,
                allow_insecure_http: false,
            };
            assert_eq!(spec.validate().is_ok(), valid);
        }
    }

    #[test]
    fn source_identity_and_permissions_cover_every_variant() {
        use citadel_primitives::ResourceType;
        let id = Uuid::now_v7();
        for (source, target, key) in [
            (
                BackupSourceSpec::CitadelSystem {},
                None,
                "citadel-system".into(),
            ),
            (
                BackupSourceSpec::Stack { stack_id: id },
                Some((ResourceType::Stack, id)),
                format!("stack:{id}"),
            ),
            (
                BackupSourceSpec::Deployment { deployment_id: id },
                Some((ResourceType::Deployment, id)),
                format!("deployment:{id}"),
            ),
            (
                BackupSourceSpec::SwarmService {
                    swarm_service_id: id,
                },
                Some((ResourceType::SwarmService, id)),
                format!("swarm-service:{id}"),
            ),
            (
                BackupSourceSpec::DockerVolume {
                    platform_id: id,
                    volume_name: "data".into(),
                    docker_node_id: Some("node-1".into()),
                    consistency: VolumeBackupConsistency::Live,
                },
                Some((ResourceType::Platform, id)),
                format!("{id}:node-1:data"),
            ),
        ] {
            source.validate().unwrap();
            assert_eq!(source.resource(), target);
            assert_eq!(source.key(), key);
            let wire = serde_json::to_value(&source).unwrap();
            assert_eq!(wire["$type"], source.kind());
            assert_eq!(
                serde_json::from_value::<BackupSourceSpec>(wire).unwrap(),
                source
            );
        }
        for source in [
            BackupSourceSpec::Stack {
                stack_id: Uuid::nil(),
            },
            BackupSourceSpec::Deployment {
                deployment_id: Uuid::nil(),
            },
            BackupSourceSpec::SwarmService {
                swarm_service_id: Uuid::nil(),
            },
            BackupSourceSpec::DockerVolume {
                platform_id: Uuid::nil(),
                volume_name: "data".into(),
                docker_node_id: None,
                consistency: VolumeBackupConsistency::Live,
            },
        ] {
            assert!(source.validate().is_err());
        }
    }

    #[test]
    fn volume_identity_matches_the_trimmed_execution_target() {
        let platform = Uuid::now_v7();
        assert_eq!(
            docker_volume_source_key(platform, Some(" node "), " data "),
            format!("{platform}:node:data")
        );
        assert_eq!(
            docker_volume_source_key(platform, Some("  "), "data"),
            docker_volume_source_key(platform, None, "data")
        );
    }
}
