//! Restricted node operations. Validate helper requests before any Docker mutation.
use citadel_contracts::citadel::{
    containers::v1::{CreateContainerRequest, ExecBinaryRequest},
    edge::v1::EdgeCommandKind,
    shared_models::v1::Mount,
    volumes::v1::CreateVolumeRequest,
};
use std::collections::HashMap;
use uuid::Uuid;

pub(super) const HELPER_BINARY: &str = "/usr/local/bin/citadel-volume-helper";
pub(super) const COMPAT_HELPER_BINARY: &str = "/app/Citadel.Agent.VolumeHelper";

pub(super) fn allowed(kind: EdgeCommandKind) -> bool {
    use EdgeCommandKind::*;
    matches!(
        kind,
        PlatformCheckHealth
            | PlatformGetInfo
            | PlatformStatsStream
            | PlatformDaemonEventsStream
            | ContainerList
            | ContainerLogsStream
            | ContainerInspect
            | ContainerCreate
            | ContainerStart
            | ContainerStop
            | ContainerPause
            | ContainerUnpause
            | ContainerRestart
            | ContainerDelete
            | ContainerStatsStream
            | ContainersStatsStream
            | ContainerExec
            | ContainerExecBinary
            | ImageList
            | ImageInspect
            | VolumeList
            | VolumeInspect
            | VolumeCreate
            | VolumeDelete
            | NetworkList
            | NetworkInspect
    )
}
fn label(labels: &HashMap<String, String>, key: &str) -> bool {
    labels
        .get(key)
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
}
fn owner(labels: &HashMap<String, String>, platform: Uuid, key: &str) -> bool {
    labels.get(key).and_then(|v| Uuid::parse_str(v).ok()) == Some(platform)
        && ["citadel.platform-id", "com.citadel.platform-id"]
            .iter()
            .all(|key| {
                labels
                    .get(*key)
                    .is_none_or(|v| Uuid::parse_str(v).ok() == Some(platform))
            })
}
fn binary(value: &str) -> bool {
    matches!(value, HELPER_BINARY | COMPAT_HELPER_BINARY)
}
fn named_volume(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
        && value != "."
        && value != ".."
}
fn mount(value: &Mount, target: &str, read_only: bool) -> bool {
    value.r#type.as_deref() == Some("volume")
        && value.source.as_deref().is_some_and(named_volume)
        && value.target.as_deref() == Some(target)
        && value.read_only == Some(read_only)
        && value.bind_options.is_none()
        && value.volume_options.is_none()
        && value.consistency.as_deref().is_none_or(str::is_empty)
}
fn base(r: &CreateContainerRequest) -> bool {
    !r.image_id.trim().is_empty()
        && r.user.as_deref() == Some("0")
        && r.privileged != Some(true)
        && r.env_vars.is_empty()
        && r.ports.is_empty()
        && r.volumes.is_empty()
        && r.networks.is_empty()
        && r.cap_drop == ["ALL"]
        && r.security_opt == ["no-new-privileges"]
        && matches!(r.restart_policy, 0 | 1)
}
pub(super) fn volume_helper(r: &CreateContainerRequest, platform: Uuid) -> bool {
    base(r)
        && label(&r.labels, "citadel.volume-browser")
        && !label(&r.labels, "citadel.backup-helper")
        && owner(&r.labels, platform, "com.citadel.platform-id")
        && r.name.starts_with("citadel-volume-helper-")
        && r.readonly_rootfs == Some(true)
        && r.network_mode.as_deref() == Some("none")
        && r.entry_point.len() == 1
        && binary(&r.entry_point[0])
        && r.command == ["volume-helper", "idle"]
        && r.cap_add == ["DAC_READ_SEARCH"]
        && r.mounts.len() == 1
        && mount(&r.mounts[0], "/data", true)
}
pub(super) fn backup_helper(r: &CreateContainerRequest, platform: Uuid) -> bool {
    base(r)
        && label(&r.labels, "citadel.backup-helper")
        && !label(&r.labels, "citadel.volume-browser")
        && owner(&r.labels, platform, "citadel.platform-id")
        && r.name.starts_with("citadel-backup-helper-")
        && r.working_dir.as_deref() == Some("/tmp")
        && r.memory_limit == Some(512 * 1024 * 1024)
        && r.memory_swap == r.memory_limit
        && r.pids_limit == Some(128)
        && r.auto_remove == Some(true)
        && r.readonly_rootfs != Some(true)
        && r.network_mode.as_deref().is_none_or(str::is_empty)
        && r.entry_point == ["/bin/sh"]
        && r.command.len() == 2
        && r.command[0] == "-c"
        && r.command[1]
            .strip_prefix("trap 'exit 0' TERM INT; sleep ")
            .and_then(|v| v.parse::<u64>().ok())
            .is_some_and(|v| (300..=86700).contains(&v))
        && r.cap_add.len() == 2
        && r.cap_add.iter().any(|v| v == "DAC_READ_SEARCH")
        && r.cap_add.iter().any(|v| v == "FOWNER")
        && (r.mounts.is_empty()
            || (r.mounts.len() == 1
                && (mount(&r.mounts[0], "/source", true) || mount(&r.mounts[0], "/target", false))))
}
pub(super) fn volume_exec(r: &ExecBinaryRequest) -> bool {
    let c = &r.cmd;
    !r.tty
        && r.env.is_empty()
        && c.len() >= 7
        && binary(&c[0])
        && c[1] == "volume-helper"
        && c[3] == "--root"
        && c[4] == "/data"
        && c[5] == "--path"
        && !c[6].is_empty()
        && citadel_platforms::volume_content::normalize_path(Some(&c[6])).is_ok()
        && match c[2].as_str() {
            "inspect" | "stream-directory" | "stream-file" => c.len() == 7,
            "list" => {
                c.len() == 11
                    && c[7..] == ["--max-entries", "1000", "--max-payload-bytes", "1048576"]
            }
            _ => false,
        }
}
pub(super) fn backup_exec(r: &ExecBinaryRequest) -> bool {
    if r.tty
        || !(2..=256).contains(&r.cmd.len())
        || r.cmd[0] != "restic"
        || ![
            "RESTIC_REPOSITORY",
            "RESTIC_PASSWORD",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
        ]
        .iter()
        .all(|k| r.env.contains_key(*k))
        || !r.env.keys().all(|k| {
            matches!(
                k.as_str(),
                "RESTIC_REPOSITORY"
                    | "RESTIC_PASSWORD"
                    | "AWS_ACCESS_KEY_ID"
                    | "AWS_SECRET_ACCESS_KEY"
                    | "AWS_SESSION_TOKEN"
                    | "AWS_DEFAULT_REGION"
            )
        })
    {
        return false;
    }
    let mut i = 1;
    while r.cmd.get(i).is_some_and(|v| v == "-o") {
        i += 1;
        if !r.cmd.get(i).is_some_and(|v| {
            matches!(
                v.as_str(),
                "s3.bucket-lookup=auto" | "s3.bucket-lookup=path" | "s3.bucket-lookup=dns"
            )
        }) {
            return false;
        }
        i += 1;
    }
    if !r
        .env
        .get("RESTIC_REPOSITORY")
        .is_some_and(|v| v.starts_with("s3:"))
    {
        return false;
    }
    let Some(operation) = r.cmd.get(i).map(String::as_str) else {
        return false;
    };
    let mut positional = Vec::new();
    let mut target = false;
    i += 1;
    while i < r.cmd.len() {
        match r.cmd[i].as_str() {
            "--json" => {}
            "--prune" if operation == "forget" => {}
            "--delete" if operation == "restore" => {}
            "--read-data-subset=1/100" if operation == "check" => {}
            "--tag" if matches!(operation, "backup" | "forget" | "snapshots") => {
                i += 1;
                if !r
                    .cmd
                    .get(i)
                    .is_some_and(|v| !v.is_empty() && !v.starts_with('-'))
                {
                    return false;
                }
            }
            "--keep-last" if operation == "forget" => {
                i += 1;
                if !r
                    .cmd
                    .get(i)
                    .and_then(|v| v.parse::<u32>().ok())
                    .is_some_and(|v| v > 0)
                {
                    return false;
                }
            }
            "--target" if operation == "restore" && !target => {
                i += 1;
                if r.cmd.get(i).map(String::as_str) != Some("/target") {
                    return false;
                }
                target = true;
            }
            value if !value.starts_with('-') => positional.push(value),
            _ => return false,
        }
        i += 1;
    }
    match operation {
        "backup" => positional == ["/source"],
        "restore" => target && positional.len() == 1,
        "snapshots" => positional.len() <= 1,
        "init" | "check" | "forget" => positional.is_empty(),
        _ => false,
    }
}

pub(super) fn restore_owner(labels: &HashMap<String, String>, platform: Uuid) -> bool {
    owner(labels, platform, "citadel.platform-id")
        && labels
            .get("citadel.backup.restoreRunId")
            .and_then(|v| Uuid::parse_str(v).ok())
            .is_some_and(|v| !v.is_nil())
}
pub(super) fn restore_create(r: &CreateVolumeRequest, platform: Uuid) -> bool {
    named_volume(&r.name)
        && r.driver == "local"
        && r.options.is_empty()
        && r.labels.len() == 2
        && restore_owner(&r.labels, platform)
}

#[cfg(test)]
pub(super) fn volume_request(platform: Uuid) -> CreateContainerRequest {
    CreateContainerRequest {
        name: "citadel-volume-helper-fixture".into(),
        image_id: "fixture:latest".into(),
        user: Some("0".into()),
        readonly_rootfs: Some(true),
        privileged: Some(false),
        network_mode: Some("none".into()),
        auto_remove: Some(true),
        labels: HashMap::from([
            ("citadel.volume-browser".into(), "true".into()),
            ("com.citadel.platform-id".into(), platform.to_string()),
        ]),
        entry_point: vec![COMPAT_HELPER_BINARY.into()],
        command: vec!["volume-helper".into(), "idle".into()],
        cap_add: vec!["DAC_READ_SEARCH".into()],
        cap_drop: vec!["ALL".into()],
        security_opt: vec!["no-new-privileges".into()],
        mounts: vec![Mount {
            r#type: Some("volume".into()),
            source: Some("fixture".into()),
            target: Some("/data".into()),
            read_only: Some(true),
            ..Default::default()
        }],
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allowlist_matches_every_reviewed_protocol_operation() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../reports/agent-parity.md"
        ));
        let mut count = 0;
        for row in source.lines().filter(|v| v.starts_with("| citadel.")) {
            let columns: Vec<_> = row.split('|').map(str::trim).collect();
            let kind = EdgeCommandKind::try_from(columns[6].parse::<i32>().unwrap()).unwrap();
            assert_eq!(allowed(kind), columns[7] != "denied", "{kind:?}");
            count += 1;
        }
        assert_eq!(count, 68);
        assert!(!allowed(EdgeCommandKind::Unspecified));
    }
    #[test]
    fn helper_labels_cannot_bypass_mount_privilege_or_command_constraints() {
        let platform = Uuid::now_v7();
        let good = volume_request(platform);
        assert!(volume_helper(&good, platform));
        let mutations: &[fn(&mut CreateContainerRequest)] = &[
            |r| r.privileged = Some(true),
            |r| r.readonly_rootfs = Some(false),
            |r| r.network_mode = Some("host".into()),
            |r| r.entry_point = vec!["sh".into()],
            |r| r.command.push("malicious".into()),
            |r| r.volumes.push("/:/host".into()),
            |r| r.env_vars.push("LD_PRELOAD=/data/exploit".into()),
            |r| r.cap_add.push("SYS_ADMIN".into()),
            |r| r.mounts[0].r#type = Some("bind".into()),
            |r| r.mounts[0].read_only = Some(false),
            |r| r.mounts[0].source = Some("/".into()),
            |r| r.mounts[0].volume_options = Some(Default::default()),
            |r| r.mounts[0].bind_options = Some(Default::default()),
            |r| {
                r.labels
                    .insert("com.citadel.platform-id".into(), Uuid::now_v7().to_string())
                    .map(|_| ())
                    .unwrap()
            },
            |r| r.restart_policy = 2,
            |r| {
                r.labels
                    .insert("citadel.backup-helper".into(), "true".into());
            },
        ];
        for change in mutations {
            let mut bad = good.clone();
            change(&mut bad);
            assert!(!volume_helper(&bad, platform));
            assert!(!backup_helper(&bad, platform));
        }
        let mut modern = good;
        modern.entry_point = vec![HELPER_BINARY.into()];
        assert!(volume_helper(&modern, platform));
    }
    #[test]
    fn binary_helper_exec_limits_operations_paths_flags_and_environment() {
        let good = ExecBinaryRequest {
            cmd: [
                COMPAT_HELPER_BINARY,
                "volume-helper",
                "list",
                "--root",
                "/data",
                "--path",
                "/",
                "--max-entries",
                "1000",
                "--max-payload-bytes",
                "1048576",
            ]
            .map(str::to_owned)
            .into(),
            ..Default::default()
        };
        assert!(volume_exec(&good));
        for (index, value) in [
            (0, "sh"),
            (2, "idle"),
            (4, "/"),
            (6, "/../etc"),
            (6, ""),
            (8, "999999"),
            (10, "999999999"),
        ] {
            let mut bad = good.clone();
            bad.cmd[index] = value.into();
            assert!(!volume_exec(&bad));
        }
        let mut bad = good.clone();
        bad.env.insert("LD_PRELOAD".into(), "evil".into());
        assert!(!volume_exec(&bad));
        let mut bad = good;
        bad.tty = true;
        assert!(!volume_exec(&bad));
    }
    #[test]
    fn backup_helpers_and_restore_volumes_require_exact_ownership_and_bounds() {
        let platform = Uuid::now_v7();
        let mut helper = volume_request(platform);
        helper.name = "citadel-backup-helper-fixture".into();
        helper.labels = HashMap::from([
            ("citadel.backup-helper".into(), "true".into()),
            ("citadel.platform-id".into(), platform.to_string()),
        ]);
        helper.working_dir = Some("/tmp".into());
        helper.memory_limit = Some(512 * 1024 * 1024);
        helper.memory_swap = helper.memory_limit;
        helper.pids_limit = Some(128);
        helper.readonly_rootfs = Some(false);
        helper.network_mode = None;
        helper.entry_point = vec!["/bin/sh".into()];
        helper.command = vec!["-c".into(), "trap 'exit 0' TERM INT; sleep 300".into()];
        helper.cap_add.push("FOWNER".into());
        helper.mounts.clear();
        assert!(backup_helper(&helper, platform));
        for lifetime in ["299", "86701", "300; touch /tmp/exploit"] {
            let mut bad = helper.clone();
            bad.command[1] = format!("trap 'exit 0' TERM INT; sleep {lifetime}");
            assert!(!backup_helper(&bad, platform));
        }
        let mut request = ExecBinaryRequest {
            cmd: vec![
                "restic".into(),
                "-o".into(),
                "s3.bucket-lookup=path".into(),
                "restore".into(),
                "snapshot".into(),
                "--target".into(),
                "/target".into(),
            ],
            env: [
                "RESTIC_REPOSITORY",
                "RESTIC_PASSWORD",
                "AWS_ACCESS_KEY_ID",
                "AWS_SECRET_ACCESS_KEY",
            ]
            .map(|k| (k.into(), "fixture".into()))
            .into(),
            ..Default::default()
        };
        request.env.insert(
            "RESTIC_REPOSITORY".into(),
            "s3:http://fixture/bucket".into(),
        );
        assert!(backup_exec(&request));
        request.cmd.push("--password-command=sh".into());
        assert!(!backup_exec(&request));
        request.cmd.pop();
        request
            .env
            .insert("RESTIC_PASSWORD_COMMAND".into(), "sh".into());
        assert!(!backup_exec(&request));
        let good = CreateVolumeRequest {
            name: "restore".into(),
            driver: "local".into(),
            labels: HashMap::from([
                ("citadel.platform-id".into(), platform.to_string()),
                (
                    "citadel.backup.restoreRunId".into(),
                    Uuid::now_v7().to_string(),
                ),
            ]),
            ..Default::default()
        };
        assert!(restore_create(&good, platform));
        assert!(!restore_create(&good, Uuid::now_v7()));
        let mut bad = good;
        bad.options.insert("device".into(), "/".into());
        assert!(!restore_create(&bad, platform));
    }
}
