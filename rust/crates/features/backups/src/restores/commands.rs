use crate::*;

#[derive(Debug, Clone)]
pub struct BackupRestoreRequest {
    pub actor: ActorId,
    pub backup_run_id: Uuid,
    pub target_platform_id: Uuid,
    pub target_volume_name: String,
    pub overwrite_existing: bool,
    pub target_docker_node_id: Option<String>,
    pub source_backup_run_item_id: Option<Uuid>,
}
