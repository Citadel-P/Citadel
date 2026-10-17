use citadel_platforms::network_usage::*;
use uuid::Uuid;

enum_schema!(
    NetworkRelationshipSchema,
    "NetworkRelationship",
    NetworkRelationship,
    [Attachment, ServiceNetwork, SavedReference]
);
enum_schema!(
    NetworkWorkloadKindSchema,
    "NetworkWorkloadKind",
    NetworkWorkloadKind,
    [Deployment, Stack, SwarmService]
);
schema_model! { NetworkUsageEntry =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=NetworkUsageEntry)]
    pub struct NetworkUsageEntrySchema {
        #[schema(value_type=NetworkRelationshipSchema)]
        pub relationship: NetworkRelationship,
        pub docker_service_id: Option<String>,
        pub service_name: Option<String>,
        pub observed_at: Option<i64>,
        pub is_stale: bool,
        pub container_id: Option<Uuid>,
        pub docker_container_id: Option<String>,
        pub docker_node_id: Option<String>,
        pub container_name: Option<String>,
        pub container_state: Option<String>,
        #[schema(value_type=Option<NetworkWorkloadKindSchema>)]
        pub workload_kind: Option<NetworkWorkloadKind>,
        pub workload_id: Option<Uuid>,
        pub workload_name: Option<String>,
        pub workload_status: Option<String>,
        pub workload_processing: bool,
        pub reference: Option<String>,
    }
}
schema_model! { NetworkUsage =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=NetworkUsage)]
    pub struct NetworkUsageSchema {
        #[schema(value_type=Vec<NetworkUsageEntrySchema>)]
        pub entries: Vec<NetworkUsageEntry>,
        pub is_partial: bool,
        pub truncated: bool,
    }
}

schema_model! { NetworkContainer =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all="camelCase")]
    #[schema(as=NetworkContainer)]
    pub struct NetworkContainerSchema {
        pub container_id: Uuid,
        pub docker_container_id: String,
        pub docker_node_id: Option<String>,
        pub container_name: String,
        pub container_state: Option<String>,
        pub is_stale: bool,
        #[schema(value_type=Option<NetworkWorkloadKindSchema>)]
        pub workload_kind: Option<NetworkWorkloadKind>,
        pub workload_id: Option<Uuid>,
        pub workload_name: Option<String>,
        pub workload_status: Option<String>,
        pub workload_processing: bool,
    }
}
