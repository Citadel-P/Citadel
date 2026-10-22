//! Restricted node operations. Validate helper requests before any Docker mutation.
use citadel_contracts::citadel::{
    containers::v1::{CreateContainerRequest, ExecBinaryRequest},
    edge::v1::EdgeCommandKind,
    shared_models::v1::Mount,
    volumes::v1::CreateVolumeRequest,
};
use citadel_docker::bind_mount::BIND_BACKUP_SCRIPT;
use citadel_docker::bind_mount::BIND_PATH_PROBE;
use citadel_docker::bind_mount::BIND_RESTORE_SCRIPT;
use citadel_docker::bind_mount::validate_bind_path;
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
            | ContainerProcesses
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
fn bind_mount(value: &Mount, target: &str, read_only: bool, probe: bool) -> bool {
    value.r#type.as_deref() == Some("bind")
        && value.source.as_deref().is_some_and(|path| {
            if probe {
                path == "/"
            } else {
                validate_bind_path(path).is_ok()
            }
        })
        && value.target.as_deref() == Some(target)
        && value.read_only == Some(read_only)
        && value.volume_options.is_none()
        && value.consistency.as_deref().is_none_or(str::is_empty)
        && value.bind_options.as_ref().is_some_and(|options| {
            options.propagation.as_deref() == Some("rprivate")
                && options.non_recursive.unwrap_or(false) != probe
                && options.create_mountpoint != Some(true)
                && options.read_only_non_recursive != Some(true)
                && options.read_only_force_recursive != Some(true)
        })
}
fn backup_mount(value: &Mount, target: &str, read_only: bool) -> bool {
    mount(value, target, read_only) || bind_mount(value, target, read_only, false)
}
fn host_probe(r: &CreateContainerRequest) -> bool {
    r.mounts.len() == 1
        && bind_mount(&r.mounts[0], "/source", true, true)
        && r.readonly_rootfs == Some(true)
        && r.network_mode.as_deref() == Some("none")
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
// Full-volume snapshots record /source or /data. Mount the destination at
// that child of /target so Restic restores the root directory metadata too.
// Bind-mount helpers retain their existing exact /target restriction.
fn restore_mount(m: &Mount) -> bool {
    backup_mount(m, "/target", false)
        || mount(m, "/target/source", false)
        || mount(m, "/target/data", false)
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
        && (host_probe(r)
            || (r.readonly_rootfs != Some(true)
                && r.network_mode.as_deref().is_none_or(str::is_empty)))
        && r.entry_point == ["/bin/sh"]
        && r.command.len() == 2
        && r.command[0] == "-c"
        && r.command[1]
            .strip_prefix("trap 'exit 0' TERM INT; sleep ")
            .and_then(|v| v.parse::<u64>().ok())
            .is_some_and(|v| (300..=86700).contains(&v))
        && (r.cap_add.len() == 2
            || (r.cap_add.len() == 3
                && r.cap_add.iter().any(|v| v == "CHOWN")
                && r.mounts.len() == 1
                && restore_mount(&r.mounts[0])))
        && r.cap_add.iter().any(|v| v == "DAC_READ_SEARCH")
        && r.cap_add.iter().any(|v| v == "FOWNER")
        && (host_probe(r)
            || r.mounts.is_empty()
            || (r.mounts.len() == 1
                && (backup_mount(&r.mounts[0], "/source", true) || restore_mount(&r.mounts[0]))))
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
    if operation == "prune" {
        return r.cmd.len() == i + 1;
    }
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
            "/../../docs/agent-protocol.md"
        ));
        let mut count = 0;
        for row in source.lines().filter(|v| v.starts_with("| citadel.")) {
            let columns: Vec<_> = row.split('|').map(str::trim).collect();
            let kind = EdgeCommandKind::try_from(columns[6].parse::<i32>().unwrap()).unwrap();
            assert_eq!(allowed(kind), columns[7] != "denied", "{kind:?}");
            count += 1;
        }
        assert_eq!(count, 69);
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
        let mut restore = helper.clone();
        restore.mounts = volume_request(platform).mounts;
        restore.mounts[0].target = Some("/target".into());
        restore.mounts[0].read_only = Some(false);
        restore.cap_add.push("CHOWN".into());
        assert!(backup_helper(&restore, platform));
        for root in ["/target/source", "/target/data"] {
            let mut rooted = restore.clone();
            rooted.mounts[0].target = Some(root.into());
            assert!(backup_helper(&rooted, platform));
            rooted.mounts[0].read_only = Some(true);
            assert!(!backup_helper(&rooted, platform));
            rooted.mounts[0].read_only = Some(false);
            rooted.mounts[0].r#type = Some("bind".into());
            assert!(!backup_helper(&rooted, platform));
        }
        for target in [
            "/target/other",
            "/target/source/child",
            "/target/../source",
            "/",
        ] {
            let mut bad = restore.clone();
            bad.mounts[0].target = Some(target.into());
            assert!(!backup_helper(&bad, platform));
        }
        let mut bad = restore.clone();
        bad.mounts.clear();
        assert!(!backup_helper(&bad, platform));
        let mut bad = restore.clone();
        bad.mounts[0].target = Some("/source".into());
        bad.mounts[0].read_only = Some(true);
        assert!(!backup_helper(&bad, platform));
        let mut bad = restore;
        bad.cap_add[2] = "SYS_ADMIN".into();
        assert!(!backup_helper(&bad, platform));
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
        let mut prune = request.clone();
        prune.cmd = vec!["restic".into(), "prune".into()];
        assert!(backup_exec(&prune));
        for extra in [
            vec!["--keep-last", "14"],
            vec!["--prune"],
            vec!["/source"],
            vec!["--password-command=sh"],
            vec!["--json"],
        ] {
            let mut bad = prune.clone();
            bad.cmd.extend(extra.into_iter().map(str::to_owned));
            assert!(!backup_exec(&bad));
        }

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

/// Shell is admitted only for these byte-for-byte templates and bounded arguments.
pub(super) fn bind_script_exec(r: &ExecBinaryRequest) -> bool {
    let c = &r.cmd;
    if r.tty || c.len() < 5 || c[0..2] != ["/bin/sh", "-c"] || c[3] != "sh" {
        return false;
    }
    if c[2] == BIND_PATH_PROBE {
        return c.len() == 5 && r.env.is_empty() && validate_bind_path(&c[4]).is_ok();
    }
    let mut restic = r.clone();
    if c[2] == BIND_BACKUP_SCRIPT.replace("/data", "/source") && c[4] == "backup" {
        restic.cmd = std::iter::once("restic".into())
            .chain(c[4..].iter().cloned())
            .collect();
    } else if c[2] == BIND_RESTORE_SCRIPT
        && c.len() == 6
        && (8..=64).contains(&c[4].len())
        && c[4].bytes().all(|c| c.is_ascii_hexdigit())
        && matches!(c[5].as_str(), "true" | "false")
    {
        restic.cmd = vec![
            "restic".into(),
            "restore".into(),
            c[4].clone(),
            "--target".into(),
            "/target".into(),
        ];
    } else {
        return false;
    }
    backup_exec(&restic)
}

pub(super) fn backup_operation(
    r: &ExecBinaryRequest,
    helper: &CreateContainerRequest,
    platform: Uuid,
) -> bool {
    if !backup_helper(helper, platform) {
        return false;
    }
    if host_probe(helper) {
        return bind_script_exec(r) && r.cmd[2] == BIND_PATH_PROBE;
    }
    if helper
        .mounts
        .first()
        .is_some_and(|mount| mount.r#type.as_deref() == Some("bind"))
    {
        return bind_script_exec(r)
            && ((r.cmd[2] == BIND_RESTORE_SCRIPT
                && bind_mount(&helper.mounts[0], "/target", false, false))
                || (r.cmd[2] == BIND_BACKUP_SCRIPT.replace("/data", "/source")
                    && bind_mount(&helper.mounts[0], "/source", true, false)));
    }
    backup_exec(r)
}

#[cfg(test)]
mod bind_tests {
    use super::*;
    use citadel_contracts::citadel::shared_models::v1::BindOptions;

    fn helper(platform: Uuid, path: &str, restore: bool) -> CreateContainerRequest {
        let probe = path == "/";
        let mut helper = volume_request(platform);
        helper.name = "citadel-backup-helper-bind".into();
        helper.labels = HashMap::from([
            ("citadel.backup-helper".into(), "true".into()),
            ("citadel.platform-id".into(), platform.to_string()),
        ]);
        helper.working_dir = Some("/tmp".into());
        helper.memory_limit = Some(512 * 1024 * 1024);
        helper.memory_swap = helper.memory_limit;
        helper.pids_limit = Some(128);
        helper.readonly_rootfs = Some(probe);
        helper.network_mode = probe.then(|| "none".into());
        helper.entry_point = vec!["/bin/sh".into()];
        helper.command = vec!["-c".into(), "trap 'exit 0' TERM INT; sleep 420".into()];
        helper.cap_add = vec!["DAC_READ_SEARCH".into(), "FOWNER".into()];
        if restore {
            helper.cap_add.push("CHOWN".into());
        }
        helper.mounts = vec![Mount {
            r#type: Some("bind".into()),
            source: Some(path.into()),
            target: Some(if restore { "/target" } else { "/source" }.into()),
            read_only: Some(!restore),
            bind_options: Some(BindOptions {
                propagation: Some("rprivate".into()),
                non_recursive: Some(!probe),
                create_mountpoint: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        }];
        helper
    }

    fn command(script: String, args: &[&str], credentials: bool) -> ExecBinaryRequest {
        let mut cmd = vec!["/bin/sh".into(), "-c".into(), script, "sh".into()];
        cmd.extend(args.iter().map(|arg| arg.to_string()));
        ExecBinaryRequest {
            container_id: "helper".into(),
            cmd,
            env: if credentials {
                HashMap::from([
                    (
                        "RESTIC_REPOSITORY".into(),
                        "s3:http://fixture/bucket".into(),
                    ),
                    ("RESTIC_PASSWORD".into(), "fixture".into()),
                    ("AWS_ACCESS_KEY_ID".into(), "fixture".into()),
                    ("AWS_SECRET_ACCESS_KEY".into(), "fixture".into()),
                ])
            } else {
                HashMap::new()
            },
            ..Default::default()
        }
    }

    #[test]
    fn host_probe_cannot_execute_backups_or_arbitrary_shell_commands() {
        let platform = Uuid::now_v7();
        let helper = helper(platform, "/", false);
        let probe = command(BIND_PATH_PROBE.into(), &["/srv/data"], false);
        assert!(backup_helper(&helper, platform));
        assert!(backup_operation(&probe, &helper, platform));
        let backup = command(
            BIND_BACKUP_SCRIPT.replace("/data", "/source"),
            &["backup", "--json", "/source"],
            true,
        );
        assert!(!backup_operation(&backup, &helper, platform));
        let mut raw = backup.clone();
        raw.cmd = vec!["restic".into(), "backup".into(), "/source".into()];
        assert!(!backup_operation(&raw, &helper, platform));
        let mut bad = probe.clone();
        bad.cmd[2].push_str("\ntouch /source/exploit");
        assert!(!bind_script_exec(&bad));
        let mut bad = probe;
        bad.cmd[4] = "/".into();
        assert!(!bind_script_exec(&bad));
        let mut bad = helper;
        bad.network_mode = None;
        assert!(!backup_helper(&bad, platform));
    }

    #[test]
    fn bind_commands_require_exact_script_mount_direction_and_bounded_arguments() {
        let platform = Uuid::now_v7();
        let source = helper(platform, "/srv/app data", false);
        let target = helper(platform, "/srv/recovered", true);
        let backup = command(
            BIND_BACKUP_SCRIPT.replace("/data", "/source"),
            &["backup", "--json", "/source"],
            true,
        );
        let restore = command(BIND_RESTORE_SCRIPT.into(), &["abcdef12", "true"], true);
        assert!(backup_operation(&backup, &source, platform));
        assert!(backup_operation(&restore, &target, platform));
        assert!(!backup_operation(&restore, &source, platform));
        assert!(!backup_operation(&backup, &target, platform));
        let mut bad = restore.clone();
        bad.cmd[4] = "--target=/source".into();
        assert!(!bind_script_exec(&bad));
        let mut bad = restore;
        bad.env.insert("LD_PRELOAD".into(), "evil".into());
        assert!(!bind_script_exec(&bad));
        for path in ["/", "/proc", "/srv/../etc"] {
            assert!(!backup_helper(&helper(platform, path, true), platform));
        }
        let mut bad = target.clone();
        bad.mounts[0].bind_options.as_mut().unwrap().non_recursive = Some(false);
        assert!(!backup_helper(&bad, platform));
        let mut bad = target;
        bad.mounts[0]
            .bind_options
            .as_mut()
            .unwrap()
            .create_mountpoint = Some(true);
        assert!(!backup_helper(&bad, platform));
    }
}

// Current-data copying has a separate exact helper shape. Ordinary backup
// helpers keep their existing single-mount and remote-repository restrictions.
pub(super) fn current_copy_helper(r: &CreateContainerRequest, platform: Uuid) -> bool {
    use citadel_docker::current_copy::COPY_JOB_LABEL;
    let Some(job) = r
        .labels
        .get(COPY_JOB_LABEL)
        .and_then(|s| Uuid::parse_str(s).ok())
    else {
        return false;
    };
    base(r)
        && owner(&r.labels, platform, "citadel.platform-id")
        && label(&r.labels, "citadel.backup-helper")
        && !label(&r.labels, "citadel.volume-browser")
        && r.name == format!("citadel-backup-helper-copy-{}", job.simple())
        && r.working_dir.as_deref() == Some("/tmp")
        && r.auto_remove == Some(true)
        && r.memory_limit == Some(512 * 1024 * 1024)
        && r.memory_swap == r.memory_limit
        && r.pids_limit == Some(128)
        && r.readonly_rootfs != Some(true)
        && r.network_mode.as_deref() == Some("none")
        && r.entry_point == ["/bin/sh"]
        && r.command == ["-c", "trap 'exit 0' TERM INT; sleep 14700"]
        && r.cap_add.len() == 3
        && ["DAC_READ_SEARCH", "FOWNER", "CHOWN"]
            .iter()
            .all(|c| r.cap_add.iter().any(|v| v == c))
        && r.mounts.len() == 2
        && mount(&r.mounts[0], "/source", true)
        && mount(&r.mounts[1], "/target/source", false)
        && r.mounts[0].source != r.mounts[1].source
}
pub(super) fn current_copy_exec(r: &ExecBinaryRequest) -> bool {
    use citadel_docker::current_copy::CURRENT_COPY_SCRIPT;
    !r.tty
        && r.cmd == ["/bin/sh", "-ec", CURRENT_COPY_SCRIPT]
        && r.env.len() == 2
        && r.env.get("RESTIC_REPOSITORY").map(String::as_str) == Some("/tmp/citadel-copy")
        && r.env
            .get("RESTIC_PASSWORD")
            .is_some_and(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
mod current_copy_tests {
    use super::*;
    use citadel_docker::current_copy::COPY_JOB_LABEL;
    use citadel_docker::current_copy::CURRENT_COPY_SCRIPT;

    #[test]
    fn current_copy_requires_isolated_read_only_source_and_exact_script() {
        let platform = Uuid::now_v7();
        let job = Uuid::now_v7();
        let mut helper = volume_request(platform);
        helper.name = format!("citadel-backup-helper-copy-{}", job.simple());
        helper.labels = HashMap::from([
            ("citadel.backup-helper".into(), "true".into()),
            ("citadel.platform-id".into(), platform.to_string()),
            (COPY_JOB_LABEL.into(), job.to_string()),
        ]);
        helper.working_dir = Some("/tmp".into());
        helper.memory_limit = Some(512 * 1024 * 1024);
        helper.memory_swap = helper.memory_limit;
        helper.pids_limit = Some(128);
        helper.readonly_rootfs = Some(false);
        helper.entry_point = vec!["/bin/sh".into()];
        helper.command = vec!["-c".into(), "trap 'exit 0' TERM INT; sleep 14700".into()];
        helper.cap_add = vec!["DAC_READ_SEARCH".into(), "FOWNER".into(), "CHOWN".into()];
        helper.mounts[0].target = Some("/source".into());
        let mut target = helper.mounts[0].clone();
        target.target = Some("/target/source".into());
        target.source = Some("new-owned-volume".into());
        target.read_only = Some(false);
        helper.mounts.push(target);
        assert!(current_copy_helper(&helper, platform));
        assert!(!current_copy_helper(&helper, Uuid::now_v7()));
        for mutation in 0..8 {
            let mut bad = helper.clone();
            match mutation {
                0 => bad.mounts[0].read_only = Some(false),
                1 => bad.mounts[1].source = bad.mounts[0].source.clone(),
                2 => bad.mounts[0].r#type = Some("bind".into()),
                3 => bad.network_mode = Some("host".into()),
                4 => bad.cap_add.push("SYS_ADMIN".into()),
                5 => bad.command[1] = "sleep infinity".into(),
                6 => bad
                    .labels
                    .insert(COPY_JOB_LABEL.into(), Uuid::now_v7().to_string())
                    .map(|_| ())
                    .unwrap(),
                7 => bad.mounts[1].target = Some("/source".into()),
                _ => unreachable!(),
            }
            assert!(!current_copy_helper(&bad, platform), "mutation {mutation}");
        }
        let exec = ExecBinaryRequest {
            cmd: vec!["/bin/sh".into(), "-ec".into(), CURRENT_COPY_SCRIPT.into()],
            env: HashMap::from([
                ("RESTIC_REPOSITORY".into(), "/tmp/citadel-copy".into()),
                ("RESTIC_PASSWORD".into(), "a1".repeat(32)),
            ]),
            ..Default::default()
        };
        assert!(current_copy_exec(&exec));
        assert!(!backup_exec(&exec));
        let mut bad = exec.clone();
        bad.cmd[2].push_str("; rm -rf /source");
        assert!(!current_copy_exec(&bad));
        let mut bad = exec.clone();
        bad.env.insert("RESTIC_REPOSITORY".into(), "/target".into());
        assert!(!current_copy_exec(&bad));
        let mut bad = exec;
        bad.env.insert("BASH_ENV".into(), "/source/script".into());
        assert!(!current_copy_exec(&bad));
    }
}
