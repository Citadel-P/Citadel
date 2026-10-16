use chrono::{DateTime, Utc};
use citadel_platforms::image_usage::*;
use uuid::Uuid;

enum_schema!(
    ImageRelationshipSchema,
    "ImageRelationship",
    citadel_platforms::image_usage::ImageRelationship,
    [Container, SavedLocalImage, SavedDigest, SavedTag]
);
enum_schema!(
    ImageWorkloadKindSchema,
    "ImageWorkloadKind",
    citadel_platforms::image_usage::ImageWorkloadKind,
    [Deployment, Stack, SwarmService]
);

schema_model! { ImageDeletionReview =>
    #[derive(serde::Serialize,utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=ImageDeletionReview)]
    pub struct ImageDeletionReviewSchema {
        pub docker_image_id: String,
        pub row_version: i64,
        pub tags: Vec<String>,
    }
}
schema_model! { ImageUsageEntry =>
    #[derive(serde::Serialize,utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=ImageUsageEntry)]
    pub struct ImageUsageEntrySchema {
        #[schema(value_type=ImageRelationshipSchema)]
        pub relationship: ImageRelationship,
        #[schema(value_type=Option<ImageWorkloadKindSchema>)]
        pub workload_kind: Option<ImageWorkloadKind>,
        pub workload_id: Option<Uuid>,
        pub workload_name: Option<String>,
        pub workload_status: Option<String>,
        pub container_id: Option<String>,
        pub container_name: Option<String>,
        pub container_state: Option<String>,
        pub reference: Option<String>,
        pub observed_at: Option<DateTime<Utc>>,
        pub is_stale: bool,
    }
}
schema_model! { ImageUsage =>
    #[derive(serde::Serialize,utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=ImageUsage)]
    pub struct ImageUsageSchema {
        #[schema(value_type=Vec<ImageUsageEntrySchema>)]
        pub entries: Vec<ImageUsageEntry>,
        pub next_offset: Option<u32>,
        pub coverage_complete: bool,
        pub limitations: Vec<String>,
    }
}
schema_model! { ImageDeletionPreviewItem =>
    #[derive(serde::Serialize,utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=ImageDeletionPreviewItem)]
    pub struct ImageDeletionPreviewItemSchema {
        #[schema(value_type=ImageDeletionReviewSchema)]
        pub review: ImageDeletionReview,
        pub can_delete: bool,
        pub blockers: Vec<String>,
        pub warnings: Vec<String>,
        #[schema(value_type=ImageUsageSchema)]
        pub usage: ImageUsage,
        pub observed_at: DateTime<Utc>,
    }
}

schema_model! { ImageUsageSummary =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=ImageUsageSummary)]
    pub struct ImageUsageSummarySchema {
        pub containers: u32,
        pub deployments: u32,
        pub stacks: u32,
        pub swarm_services: u32,
        pub is_partial: bool,
    }
}
enum_schema!(
    ImageUpdateStatusSchema,
    "ImageUpdateStatus",
    citadel_platforms::image_updates::ImageUpdateStatus,
    [
        UpToDate,
        UpdateAvailable,
        NotChecked,
        CheckFailed,
        NotApplicable,
        Unknown
    ]
);
schema_model! { citadel_platforms::image_updates::ImageTagUpdate =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=ImageTagUpdate)]
    pub struct ImageTagUpdateSchema {
        pub reference: String,
        pub registry_id: Option<Uuid>,
        #[schema(value_type=ImageUpdateStatusSchema)]
        pub status: citadel_platforms::image_updates::ImageUpdateStatus,
        pub reason: Option<String>,
        pub last_attempt_at: Option<DateTime<Utc>>,
        pub last_success_at: Option<DateTime<Utc>>,
        pub local_config_digest: Option<String>,
        pub remote_config_digest: Option<String>,
        pub remote_manifest_digest: Option<String>,
        pub is_stale: bool,
    }
}
