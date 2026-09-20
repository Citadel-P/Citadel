use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum BackupPreviewKind {
    Deployment,
    Stack,
    SwarmService,
}

#[derive(Debug)]
pub enum BackupPreviewResource {
    Deployment { id: Uuid, name: String },
    Stack { id: Uuid, name: String },
    SwarmService { id: Uuid, name: String },
}

#[derive(Debug)]
pub struct BackupSourcePreview {
    pub resource: BackupPreviewResource,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub platform_status: String,
    pub volumes: Vec<BackupVolumePreview>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct BackupVolumePreview {
    pub name: String,
    pub kind: String,
    pub is_external: bool,
    pub is_shared: bool,
    pub has_backup_coverage: bool,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
}
