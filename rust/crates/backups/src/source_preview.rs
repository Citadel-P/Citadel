use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum BackupPreviewKind {
    Deployment,
    Stack,
    SwarmService,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BackupPreviewResource {
    Deployment {
        #[serde(rename = "deploymentId")]
        id: Uuid,
        #[serde(rename = "deploymentName")]
        name: String,
    },
    Stack {
        #[serde(rename = "stackId")]
        id: Uuid,
        #[serde(rename = "stackName")]
        name: String,
    },
    SwarmService {
        #[serde(rename = "swarmServiceId")]
        id: Uuid,
        #[serde(rename = "swarmServiceName")]
        name: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSourcePreview {
    #[serde(flatten)]
    pub resource: BackupPreviewResource,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub platform_status: String,
    pub volumes: Vec<BackupVolumePreview>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupVolumePreview {
    pub name: String,
    pub kind: String,
    pub is_external: bool,
    pub is_shared: bool,
    pub has_backup_coverage: bool,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
}
