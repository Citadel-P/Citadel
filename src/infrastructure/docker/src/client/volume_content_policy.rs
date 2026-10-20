//! Storage protection is evaluated by the owning daemon, including stopped
//! containers. Names and cached Core inventory are not a security boundary.
use super::{DockerClient, DockerError, projection::DockerVolume};
use citadel_docker_api::models::ContainerSummary;
use std::{collections::HashSet, path::PathBuf};

impl DockerClient {
    pub(super) async fn apply_volume_content_policy(
        &self,
        volumes: &mut [DockerVolume],
    ) -> Result<(), DockerError> {
        if volumes.is_empty() {
            return Ok(());
        }
        // One metadata-only request for the batch; no filesystem scans or
        // per-volume container inspections. all=true also protects stopped Core.
        let containers = self
            .list_container_models(Some(true), None, Some(false), None)
            .await?;
        let policy = StorageProtection::from_containers(&containers);
        for volume in volumes {
            volume.content_access_allowed = Some(policy.allows(volume));
        }
        Ok(())
    }
}

#[derive(Default)]
struct StorageProtection {
    volumes: HashSet<String>,
    bind_sources: HashSet<PathBuf>,
    incomplete: bool,
}

impl StorageProtection {
    fn from_containers(containers: &[ContainerSummary]) -> Self {
        let mut policy = Self::default();
        for container in containers {
            let Some(labels) = &container.labels else {
                continue;
            };
            let role = labels
                .get("com.citadel.system-role")
                .map(|s| s.to_ascii_lowercase());
            // Temporary helpers mount customer data and must not classify it as
            // system storage. All other explicitly marked system roles qualify.
            if matches!(role.as_deref(), Some("volume-helper" | "backup-helper")) {
                continue;
            }
            let system = labels
                .get("com.citadel.system")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"))
                || role.as_ref().is_some_and(|v| !v.is_empty());
            if !system {
                continue;
            }
            let Some(mounts) = &container.mounts else {
                policy.incomplete = true;
                continue;
            };
            for mount in mounts {
                if mount.r#type.is_none() {
                    policy.incomplete = true;
                }
                if mount.r#type == Some(citadel_docker_api::models::mount_point::Type::Volume) {
                    match mount.name.as_ref().filter(|name| !name.is_empty()) {
                        Some(name) => {
                            policy.volumes.insert(name.clone());
                        }
                        None => policy.incomplete = true,
                    }
                }
                if mount.r#type == Some(citadel_docker_api::models::mount_point::Type::Bind) {
                    // Core's documented read-only host telemetry mount does not
                    // make every workload volume system storage.
                    // Rootless installations can expose just the daemon data
                    // root at /host/<absolute source>, rather than all of '/'.
                    if mount.rw == Some(false)
                        && mount
                            .source
                            .as_deref()
                            .zip(mount.destination.as_deref())
                            .is_some_and(|(source, destination)| {
                                telemetry_mount(source, destination)
                            })
                    {
                        continue;
                    }
                    match mount.source.as_ref().map(PathBuf::from) {
                        Some(path) if path.is_absolute() => {
                            policy.bind_sources.insert(path);
                        }
                        _ => policy.incomplete = true,
                    }
                }
            }
        }
        policy
    }

    fn allows(&self, volume: &DockerVolume) -> bool {
        // Driver options can alias any host directory (including '/', Docker's
        // data root or Core's bind-mounted keys). Plugin backing stores cannot be
        // verified here either. Keep generic browsing to plain local volumes.
        !self.incomplete
            && volume.driver == "local"
            && volume.options.is_empty()
            && !volume
                .labels
                .get("com.citadel.system")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"))
            && !volume
                .labels
                .get("com.citadel.system-role")
                .is_some_and(|v| !v.is_empty())
            && !self.volumes.contains(&volume.name)
            && (self.bind_sources.is_empty()
                || (std::path::Path::new(&volume.mountpoint).is_absolute()
                    && !self.bind_sources.iter().any(|source| {
                        let location = std::path::Path::new(&volume.mountpoint);
                        location.starts_with(source) || source.starts_with(location)
                    })))
    }
}

fn telemetry_mount(source: &str, destination: &str) -> bool {
    let path = std::path::Path::new(source);
    path.is_absolute()
        && !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        && path.strip_prefix("/").is_ok_and(|relative| {
            std::path::Path::new(destination) == std::path::Path::new("/host").join(relative)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn volume(name: &str) -> DockerVolume {
        DockerVolume {
            name: name.into(),
            driver: "local".into(),
            mountpoint: format!("/var/lib/docker/volumes/{name}/_data"),
            ..Default::default()
        }
    }

    #[test]
    fn system_mounts_are_protected_regardless_of_names_state_or_role_casing() {
        for role in [
            "core",
            "database",
            "agent",
            "edge-agent",
            "swarm-node-agent",
            "CORE",
        ] {
            let container = serde_json::from_value(json!({
                "State": "exited", "Labels": {"com.citadel.system-role": role},
                "Mounts": [{"Type":"volume", "Name":"custom-secrets", "Destination":"/app/data"},
                    {"Type":"bind", "Source":"/", "Destination":"/host", "RW":false}]
            }))
            .unwrap();
            let policy = StorageProtection::from_containers(&[container]);
            assert!(!policy.allows(&volume("custom-secrets")));
            assert!(policy.allows(&volume("workload")));
            assert!(
                StorageProtection::default().allows(&volume("custom-secrets")),
                "protection is daemon-local"
            );
        }
    }

    #[test]
    fn helpers_do_not_lock_workload_volumes_and_explicit_labels_protect_detached_storage() {
        for role in ["backup-helper", "volume-helper"] {
            let container = serde_json::from_value(json!({
                "Labels":{"com.citadel.system":"true", "com.citadel.system-role":role},
                "Mounts":[{"Type":"volume", "Name":"workload"}]
            }))
            .unwrap();
            assert!(StorageProtection::from_containers(&[container]).allows(&volume("workload")));
        }
        let mut detached = volume("detached");
        detached
            .labels
            .insert("com.citadel.system".into(), "TRUE".into());
        assert!(!StorageProtection::default().allows(&detached));
    }

    #[test]
    fn system_bind_mounts_cannot_expose_the_same_data_through_a_named_volume() {
        for source in [
            "/var/lib/docker/volumes/private/_data",
            "/var/lib/docker/volumes/private/_data/keys",
        ] {
            let container = serde_json::from_value(json!({
                "Labels":{"com.citadel.system-role":"core"},
                "Mounts":[{"Type":"bind", "Source":source, "Destination":"/app/data"}]
            }))
            .unwrap();
            let policy = StorageProtection::from_containers(&[container]);
            assert!(!policy.allows(&volume("private")));
            assert!(
                policy.allows(&volume("private-other")),
                "compare path components, not prefixes"
            );
            let mut unknown = volume("unknown");
            unknown.mountpoint.clear();
            assert!(!policy.allows(&unknown));
        }
    }

    #[test]
    fn aliases_plugins_and_incomplete_inventory_fail_closed() {
        let mut alias = volume("alias");
        alias.options.insert(
            "device".into(),
            "/var/lib/docker/volumes/private/_data".into(),
        );
        assert!(!StorageProtection::default().allows(&alias));
        alias.options.clear();
        alias.driver = "plugin".into();
        assert!(!StorageProtection::default().allows(&alias));
        for mounts in [json!(null), json!([{"Type":"volume"}]), json!([{}])] {
            let container = serde_json::from_value(json!({
                "Labels":{"com.citadel.system-role":"core"}, "Mounts":mounts
            }))
            .unwrap();
            assert!(!StorageProtection::from_containers(&[container]).allows(&volume("workload")));
        }
    }

    #[test]
    fn rootless_read_only_telemetry_does_not_protect_every_workload_volume() {
        let root = "/home/citadel-demo/.local/share/docker";
        let container = serde_json::from_value(json!({
            "Labels":{"com.citadel.system-role":"core"},
            "Mounts":[
                {"Type":"bind", "Source":root, "Destination":format!("/host{root}"), "RW":false},
                {"Type":"volume", "Name":"core-data", "Destination":"/app/data"},
                {"Type":"bind", "Source":format!("{root}/volumes/private/_data"), "Destination":"/app/keys", "RW":false}
            ]
        })).unwrap();
        let policy = StorageProtection::from_containers(&[container]);
        for (name, allowed) in [
            ("example-rustfs_data", true),
            ("core-data", false),
            ("private", false),
        ] {
            let mut v = volume(name);
            v.mountpoint = format!("{root}/volumes/{name}/_data");
            assert_eq!(policy.allows(&v), allowed, "{name}");
        }
        for (destination, rw) in [
            (format!("/host{root}"), true),
            ("/host/data".into(), false),
            ("/app/data".into(), false),
        ] {
            let container = serde_json::from_value(json!({
                "Labels":{"com.citadel.system-role":"core"},
                "Mounts":[{"Type":"bind", "Source":root, "Destination":destination, "RW":rw}]
            }))
            .unwrap();
            let mut v = volume("workload");
            v.mountpoint = format!("{root}/volumes/workload/_data");
            assert!(!StorageProtection::from_containers(&[container]).allows(&v));
        }
        assert!(!telemetry_mount("relative", "/host/relative"));
        assert!(!telemetry_mount(
            "/data/../private",
            "/host/data/../private"
        ));
    }
}
