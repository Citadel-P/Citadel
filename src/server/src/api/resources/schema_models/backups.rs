//! Server-owned OpenAPI descriptions of backups values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use uuid::Uuid;

enum_schema!(
    BackupRepositoryStatusSchema,
    "BackupRepositoryStatus",
    citadel_backups::BackupRepositoryStatus,
    [Unavailable, Unknown, Uninitialized, Ready]
);

enum_schema!(
    BackupRepositoryValidationStatusSchema,
    "BackupRepositoryValidationStatus",
    citadel_backups::BackupRepositoryValidationStatus,
    [
        Unknown,
        Ready,
        Uninitialized,
        Unavailable,
        InvalidPassword,
        InvalidConfiguration
    ]
);

enum_schema!(
    BackupRestoreStatusSchema,
    "BackupRestoreStatus",
    citadel_backups::BackupRestoreStatus,
    [
        Queued,
        Preparing,
        Running,
        Succeeded,
        SucceededWithWarnings,
        Failed,
        TimedOut,
        Cancelled,
        Rejected,
        Interrupted,
        Processing
    ]
);

enum_schema!(
    BackupRunStatusSchema,
    "BackupRunStatus",
    citadel_backups::BackupRunStatus,
    [
        Queued,
        Preparing,
        Running,
        ApplyingRetention,
        Succeeded,
        SucceededWithWarnings,
        Failed,
        TimedOut,
        Cancelled,
        Rejected,
        Interrupted,
        Processing
    ]
);

enum_schema!(
    BackupRunItemStatusSchema,
    "BackupRunItemStatus",
    citadel_backups::BackupRunItemStatus,
    [Queued, Pending, Running, Succeeded, Failed, Cancelled]
);

enum_schema!(
    BackupSnapshotAvailabilitySchema,
    "BackupSnapshotAvailability",
    citadel_backups::BackupSnapshotAvailability,
    [Pending, Available, Expired, Missing, NotCreated]
);

enum_schema!(
    BackupCoverageStatusSchema,
    "BackupCoverageStatus",
    citadel_backups::BackupCoverageStatus,
    [NotApplicable, Unprotected, Protected, Warning, Failed]
);

enum_schema!(
    BackupExecutionLocationSchema,
    "BackupExecutionLocation",
    citadel_backups::spec::BackupExecutionLocation,
    [Core, Platform]
);

enum_schema!(
    S3BucketLookupSchema,
    "S3BucketLookup",
    citadel_backups::spec::S3BucketLookup,
    [Auto, Path, Dns]
);

enum_schema!(
    VolumeBackupConsistencySchema,
    "VolumeBackupConsistency",
    citadel_backups::spec::VolumeBackupConsistency,
    [Live, StopAttachedContainers]
);

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
#[schema(as = BackupRepositorySpec)]
pub enum BackupRepositorySpecSchema {
    #[serde(rename_all = "camelCase")]
    FileSystem {
        #[schema(value_type = crate::api::resources::schema_models::backups::BackupExecutionLocationSchema)]
        location: citadel_backups::spec::BackupExecutionLocation,

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
        #[schema(value_type = crate::api::resources::schema_models::backups::S3BucketLookupSchema)]
        bucket_lookup: citadel_backups::spec::S3BucketLookup,

        access_key_secret_id: Uuid,

        secret_key_secret_id: Uuid,

        session_token_secret_id: Option<Uuid>,
        #[serde(default)]
        allow_insecure_http: bool,
    },
}

impl From<citadel_backups::spec::BackupRepositorySpec> for BackupRepositorySpecSchema {
    fn from(value: citadel_backups::spec::BackupRepositorySpec) -> Self {
        match value {
            citadel_backups::spec::BackupRepositorySpec::FileSystem {
                location,
                platform_id,
                path,
            } => Self::FileSystem {
                location,
                platform_id,
                path,
            },
            citadel_backups::spec::BackupRepositorySpec::S3Compatible {
                endpoint,
                bucket,
                prefix,
                region,
                bucket_lookup,
                access_key_secret_id,
                secret_key_secret_id,
                session_token_secret_id,
                allow_insecure_http,
            } => Self::S3Compatible {
                endpoint,
                bucket,
                prefix,
                region,
                bucket_lookup,
                access_key_secret_id,
                secret_key_secret_id,
                session_token_secret_id,
                allow_insecure_http,
            },
        }
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
#[schema(as = BackupSourceSpec)]
pub enum BackupSourceSpecSchema {
    #[serde(rename_all = "camelCase")]
    DockerVolume {
        platform_id: Uuid,

        volume_name: String,

        docker_node_id: Option<String>,
        #[serde(default)]
        #[schema(value_type = crate::api::resources::schema_models::backups::VolumeBackupConsistencySchema)]
        consistency: citadel_backups::spec::VolumeBackupConsistency,
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

impl From<citadel_backups::spec::BackupSourceSpec> for BackupSourceSpecSchema {
    fn from(value: citadel_backups::spec::BackupSourceSpec) -> Self {
        match value {
            citadel_backups::spec::BackupSourceSpec::DockerVolume {
                platform_id,
                volume_name,
                docker_node_id,
                consistency,
            } => Self::DockerVolume {
                platform_id,
                volume_name,
                docker_node_id,
                consistency,
            },
            citadel_backups::spec::BackupSourceSpec::CitadelSystem {} => Self::CitadelSystem {},
            citadel_backups::spec::BackupSourceSpec::Stack { stack_id } => Self::Stack { stack_id },
            citadel_backups::spec::BackupSourceSpec::Deployment { deployment_id } => {
                Self::Deployment { deployment_id }
            }
            citadel_backups::spec::BackupSourceSpec::SwarmService { swarm_service_id } => {
                Self::SwarmService { swarm_service_id }
            }
        }
    }
}
