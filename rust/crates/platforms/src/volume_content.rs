use crate::{RuntimeCapabilityError, RuntimeErrorKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DIRECTORY_ENTRY_LIMIT: usize = 1000;
pub const DIRECTORY_PAYLOAD_LIMIT: usize = 1024 * 1024;

/// The API accepts canonical volume-relative paths, not host paths or shell input.
pub fn normalize_path(input: Option<&str>) -> Result<&str, RuntimeCapabilityError> {
    let path = input.filter(|p| !p.is_empty()).unwrap_or("/");
    if path.len() > 4096
        || !path.starts_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || (path != "/"
            && path[1..]
                .split('/')
                .any(|part| matches!(part, "" | "." | "..")))
    {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::InvalidRequest,
            "Use a canonical absolute path within the Volume.",
            false,
        ));
    }
    Ok(path)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum VolumeEntryType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct VolumeFileEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type", alias = "Type")]
    pub entry_type: VolumeEntryType,
    pub size: Option<u64>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub link_target: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeDirectory {
    pub platform_id: Uuid,
    pub volume_name: String,
    pub path: String,
    pub entries: Vec<VolumeFileEntry>,
    pub is_truncated: bool,
}
