//! Ownership proof for newly allocated restore destinations. Docker volume
//! creation is idempotent by name: a successful create is not proof of ownership.
use citadel_platforms::{CreateRuntimeVolume, RuntimeContainerSummary, RuntimeVolumeSummary};
use std::collections::BTreeMap;
use uuid::Uuid;

const PLATFORM_LABEL: &str = "citadel.platform-id";
const RESTORE_LABEL: &str = "citadel.backup.restoreRunId";

pub(crate) struct RestoreVolumeOwner {
    platform: Uuid,
    restore: Uuid,
}

impl RestoreVolumeOwner {
    pub fn new(platform: Uuid, restore: Uuid) -> Self {
        Self { platform, restore }
    }

    pub fn request(&self, name: &str) -> CreateRuntimeVolume {
        CreateRuntimeVolume {
            name: name.into(),
            driver: "local".into(),
            labels: BTreeMap::from([
                (PLATFORM_LABEL.into(), self.platform.to_string()),
                (RESTORE_LABEL.into(), self.restore.to_string()),
            ]),
            options: BTreeMap::new(),
        }
    }

    pub fn verify(&self, name: &str, volume: &RuntimeVolumeSummary) -> Result<(), String> {
        if volume.name != name
            || volume.driver != "local"
            || volume.scope != "local"
            || !volume.options.is_empty()
            || volume.labels.get(PLATFORM_LABEL) != Some(&self.platform.to_string())
            || volume.labels.get(RESTORE_LABEL) != Some(&self.restore.to_string())
        {
            return Err(Self::conflict());
        }
        Ok(())
    }

    /// Output only a boolean, never arbitrary volume labels or driver secrets.
    pub fn local_probe(&self, name: &str) -> String {
        let name = serde_json::to_string(name).expect("serializing a string cannot fail");
        format!(
            "{{{{and (eq .Name {name}) (eq .Driver \"local\") (eq .Scope \"local\") (not .Options) (eq (index .Labels \"{PLATFORM_LABEL}\") \"{}\") (eq (index .Labels \"{RESTORE_LABEL}\") \"{}\")}}}}",
            self.platform, self.restore,
        )
    }

    pub fn conflict() -> String {
        "Target Volume ownership could not be verified. No restore was started; inspect the destination before retrying.".into()
    }
}

/// Only the restore helper may reference a writable target, including stopped
/// consumers. Incomplete observations are not proof that storage is unused.
pub(crate) fn ensure_unused(
    volume: &str,
    helper: &str,
    containers: &[RuntimeContainerSummary],
) -> Result<(), String> {
    for container in containers.iter().filter(|container| container.id != helper) {
        let mounts = container
            .mounts
            .as_ref()
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "Target Volume usage could not be verified.".to_owned())?;
        for mount in mounts {
            let kind = mount
                .get("Type")
                .or_else(|| mount.get("type"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "Target Volume usage could not be verified.".to_owned())?;
            if kind != "volume" {
                continue;
            }
            let name = mount
                .get("Name")
                .or_else(|| mount.get("name"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "Target Volume usage could not be verified.".to_owned())?;
            if name == volume {
                return Err("Target Volume is in use by another container.".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_consumers_and_incomplete_mount_observations_block_restore() {
        let helper = RuntimeContainerSummary {
            id: "helper".into(),
            ..Default::default()
        };
        let mut other = RuntimeContainerSummary {
            id: "other".into(),
            state: "exited".into(),
            mounts: Some(serde_json::json!([{"Type":"volume","Name":"target"}])),
            ..Default::default()
        };
        assert!(ensure_unused("target", "helper", &[helper]).is_ok());
        assert!(ensure_unused("target", "helper", &[other.clone()]).is_err());
        assert!(ensure_unused("unrelated", "helper", &[other.clone()]).is_ok());
        other.mounts = None;
        assert!(ensure_unused("target", "helper", &[other.clone()]).is_err());
        other.mounts = Some(serde_json::json!([{"Type":"volume"}]));
        assert!(ensure_unused("target", "helper", &[other]).is_err());
    }

    #[test]
    fn creation_response_must_prove_exact_restore_and_platform_ownership() {
        let owner = RestoreVolumeOwner::new(Uuid::now_v7(), Uuid::now_v7());
        let request = owner.request("copy-data");
        let mut volume = RuntimeVolumeSummary {
            name: request.name.clone(),
            driver: request.driver,
            scope: "local".into(),
            labels: request.labels,
            options: request.options,
            ..Default::default()
        };
        assert!(owner.verify("copy-data", &volume).is_ok());
        assert!(owner.verify("different-volume", &volume).is_err());
        volume.labels.remove(RESTORE_LABEL);
        assert!(owner.verify("copy-data", &volume).is_err());
        volume.labels = owner.request("copy-data").labels;
        volume
            .labels
            .insert(PLATFORM_LABEL.into(), Uuid::now_v7().to_string());
        assert!(owner.verify("copy-data", &volume).is_err());
        volume.labels = owner.request("copy-data").labels;
        volume
            .labels
            .insert(RESTORE_LABEL.into(), Uuid::now_v7().to_string());
        assert!(owner.verify("copy-data", &volume).is_err());
        volume.labels = owner.request("copy-data").labels;
        volume.options.insert("device".into(), "/production".into());
        assert!(owner.verify("copy-data", &volume).is_err());
        volume.options.clear();
        volume.scope = "global".into();
        assert!(owner.verify("copy-data", &volume).is_err());
        volume.scope = "local".into();
        volume.driver = "remote-plugin".into();
        assert!(owner.verify("copy-data", &volume).is_err());
    }
}
