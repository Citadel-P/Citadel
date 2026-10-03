#![cfg(unix)]

use citadel_adapters::connectors::docker::DockerEndpoint;
use citadel_adapters::external::builds::runtime::{
    BuildRuntimeError, DockerBuildOptions, DockerBuildSession, DockerBuildSource,
};
use citadel_adapters::external::stacks::LocalStackApply;
use citadel_builds::{BuildArgSpec, BuildError, BuildLogSink, BuildRegistryCredentials};
use citadel_stacks::{
    StackApplySource, StackOrchestrationMode, StackReleaseStatus, StackSourceFile,
};
use futures_util::future::BoxFuture;
use std::{os::unix::fs::PermissionsExt, path::PathBuf, sync::Mutex, time::Duration};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

struct Fixture(PathBuf);
impl Fixture {
    fn new(script: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("citadel-runtime-test-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&root).unwrap();
        let executable = root.join("docker");
        std::fs::write(&executable, format!("#!/bin/sh\nset -eu\n{script}")).unwrap();
        std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn docker(&self) -> String {
        self.0.join("docker").to_str().unwrap().to_owned()
    }
    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.0.join(name)).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct Logs(Mutex<String>);
impl BuildLogSink for Logs {
    fn append<'a>(&'a self, _: &'a str, message: &'a str) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            self.0.lock().unwrap().push_str(message);
            Ok(())
        })
    }
}
fn endpoint() -> DockerEndpoint {
    "tcp://fixture-docker:2375".parse().unwrap()
}

#[tokio::test]
async fn build_streams_archive_secrets_and_push_using_private_credentials_and_selected_daemon() {
    let fixture = Fixture::new(
        r#"
root=$(dirname "$0")
test "$DOCKER_HOST" = tcp://fixture-docker:2375
test -z "$DOCKER_CONTEXT$DOCKER_TLS_VERIFY$DOCKER_CERT_PATH"
printf '%s' "$DOCKER_CONFIG" > "$root/config-path"
test "$(stat -c %a "$DOCKER_CONFIG")" = 700
case "$1" in
login)
  test "$2" = registry.example
  test "$3" = --username
  test "$4" = builder
  test "$5" = --password-stdin
  test "$(cat)" = login-password
  printf '%s' login-password > "$DOCKER_CONFIG/config.json" ;;
build)
  printf '%s\n' "$@" > "$root/build-args"
  test -f "$DOCKER_CONFIG/config.json"
  test "$DOCKER_BUILDKIT" = 1
  test "$CITADEL_BUILDKIT_SECRET_0" = build-secret
  cat > "$root/context"
  printf 'progress: %s and login-password\n' "$CITADEL_BUILDKIT_SECRET_0" >&2 ;;
push)
  printf 'digest: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n' ;;
*) exit 4 ;;
esac
"#,
    );
    let cancel = CancellationToken::new();
    let credentials = BuildRegistryCredentials {
        username: "builder".into(),
        password: Zeroizing::new("login-password".into()),
    };
    let logs = Logs::default();
    let session = DockerBuildSession::open(
        fixture.docker(),
        endpoint(),
        Some(("https://registry.example/", &credentials)),
        Duration::from_secs(3),
        4096,
        &cancel,
    )
    .await
    .unwrap();
    // The runtime forwards the archive unchanged; Docker owns its decoding.
    let archive = [0, 255, 42, 0, 1];
    session
        .build(
            DockerBuildOptions {
                source: DockerBuildSource::Archive {
                    data: &archive,
                    dockerfile: "build/Dockerfile",
                },
                tags: &["registry.example/test:latest".into()],
                build_args: &[BuildArgSpec {
                    name: "VALUE".into(),
                    value: Some("two words".into()),
                }],
                target: Some("runtime"),
                secrets: &[("token".into(), Zeroizing::new("build-secret".into()))],
            },
            &logs,
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(fixture.0.join("context")).unwrap(), archive);
    let args = fixture.read("build-args");
    assert!(args.contains("--file\nbuild/Dockerfile\n--target\nruntime\n"));
    assert!(args.contains("--build-arg\nVALUE=two words\n"));
    assert!(args.contains("--secret\nid=token,env=CITADEL_BUILDKIT_SECRET_0\n-\n"));
    let digest = session
        .push("registry.example/test:latest", &logs, &cancel)
        .await
        .unwrap();
    assert_eq!(
        digest.as_deref(),
        Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    let config = PathBuf::from(fixture.read("config-path"));
    assert!(config.exists());
    drop(session);
    assert!(!config.exists());
    let output = logs.0.lock().unwrap();
    assert!(output.contains("progress:"));
    assert!(!output.contains("build-secret"));
    assert!(!output.contains("login-password"));
}

#[tokio::test]
async fn failed_login_cleans_credentials_and_redacts_failure() {
    let fixture = Fixture::new(
        r#"
printf '%s' "$DOCKER_CONFIG" > "$(dirname "$0")/config-path"
cat >&2
exit 1
"#,
    );
    let credentials = BuildRegistryCredentials {
        username: "builder".into(),
        password: Zeroizing::new("private-password".into()),
    };
    let result = DockerBuildSession::open(
        fixture.docker(),
        endpoint(),
        Some(("registry.example", &credentials)),
        Duration::from_secs(3),
        4096,
        &CancellationToken::new(),
    )
    .await;
    let error = result.err().unwrap().to_string();
    assert!(!error.contains("private-password"));
    assert!(!PathBuf::from(fixture.read("config-path")).exists());
}

#[tokio::test]
async fn build_rejects_unsafe_dockerfile_paths_and_cancels_running_process() {
    let fixture = Fixture::new("touch \"$(dirname \"$0\")/started\"\nexec sleep 30");
    let cancel = CancellationToken::new();
    let session = DockerBuildSession::open(
        fixture.docker(),
        endpoint(),
        None,
        Duration::from_secs(40),
        4096,
        &cancel,
    )
    .await
    .unwrap();
    let logs = Logs::default();
    for dockerfile in ["../Dockerfile", "/Dockerfile", ".git/config"] {
        let result = session
            .build(
                DockerBuildOptions {
                    source: DockerBuildSource::Archive {
                        data: b"archive",
                        dockerfile,
                    },
                    tags: &[],
                    build_args: &[],
                    target: None,
                    secrets: &[],
                },
                &logs,
                &cancel,
            )
            .await;
        assert!(matches!(result, Err(BuildRuntimeError::Validation(_))));
    }
    assert!(!fixture.0.join("started").exists());
    let build = session.build(
        DockerBuildOptions {
            source: DockerBuildSource::Archive {
                data: b"archive",
                dockerfile: "Dockerfile",
            },
            tags: &[],
            build_args: &[],
            target: None,
            secrets: &[],
        },
        &logs,
        &cancel,
    );
    let stop = async {
        tokio::time::timeout(Duration::from_secs(3), async {
            while !fixture.0.join("started").exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        cancel.cancel();
    };
    let (result, ()) =
        tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(build, stop) })
            .await
            .unwrap();
    assert!(matches!(
        result,
        Err(BuildRuntimeError::Process(
            citadel_execution::ProcessError::Cancelled
        ))
    ));
}

fn stack_source() -> StackApplySource {
    StackApplySource {
        files: vec![StackSourceFile {
            relative_path: "compose.yml".into(),
            content: b"services: {}".to_vec(),
        }],
        compose_paths: vec!["compose.yml".into()],
        env_file_paths: vec![],
        working_directory: ".".into(),
        labels_override_path: None,
        resolved_commit_sha: None,
    }
}
fn stack(fixture: &Fixture, orchestration: StackOrchestrationMode) -> LocalStackApply {
    LocalStackApply {
        docker: fixture.docker(),
        endpoint: endpoint(),
        project_name: "test-stack".into(),
        orchestration,
        destroy_before_deploy: false,
        pre_deploy: None,
        post_deploy: None,
        service_names: vec![],
        pull_images: true,
    }
}

#[tokio::test]
async fn cancelling_compose_or_swarm_cleans_staged_files() {
    for mode in [
        StackOrchestrationMode::DockerCompose,
        StackOrchestrationMode::DockerSwarm,
    ] {
        let fixture = Fixture::new("pwd > \"$(dirname \"$0\")/workspace\"\nexec sleep 30");
        let cancel = CancellationToken::new();
        let runtime = stack(&fixture, mode);
        let source = stack_source();
        let apply = runtime.apply(&source, &[], None, &cancel, None);
        let stop = async {
            tokio::time::timeout(Duration::from_secs(3), async {
                while !fixture.0.join("workspace").exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            cancel.cancel();
        };
        let (result, ()) =
            tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(apply, stop) })
                .await
                .unwrap();
        assert!(matches!(result, Err(citadel_stacks::StackError::Cancelled)));
        assert!(!PathBuf::from(fixture.read("workspace").trim()).exists());
    }
}

#[tokio::test]
async fn stack_apply_stages_and_cleans_files_for_compose_and_swarm_including_failures() {
    for mode in [
        StackOrchestrationMode::DockerCompose,
        StackOrchestrationMode::DockerSwarm,
    ] {
        for fail in [false, true] {
            let fixture = Fixture::new(
                r#"
root=$(dirname "$0")
pwd > "$root/workspace"
test "$(cat compose.yml)" = 'services: {}'
test "$DOCKER_HOST" = tcp://fixture-docker:2375
test -z "$DOCKER_CONTEXT$DOCKER_TLS_VERIFY$DOCKER_CERT_PATH"
printf '%s\n' "$@" > "$root/args"
test "$1" = --config
test "$(stat -c %a "$2/config.json")" = 600
printf 'applying\n'
test "$FAIL" = false
"#,
            );
            let result = stack(&fixture, mode)
                .apply(
                    &stack_source(),
                    &[
                        format!("FAIL={fail}"),
                        "DOCKER_HOST=tcp://wrong:1234".into(),
                        "DOCKER_CONTEXT=wrong".into(),
                        "DOCKER_TLS_VERIFY=1".into(),
                    ],
                    Some(("registry.example", "fixture-auth")),
                    &CancellationToken::new(),
                    None,
                )
                .await
                .unwrap();
            assert_eq!(
                result.status,
                if fail {
                    StackReleaseStatus::Failed
                } else {
                    StackReleaseStatus::Healthy
                }
            );
            assert!(result.messages.iter().any(|item| {
                item.message
                    .as_deref()
                    .is_some_and(|value| value.contains("applying"))
            }));
            assert!(!PathBuf::from(fixture.read("workspace").trim()).exists());
            let args = fixture.read("args");
            if mode == StackOrchestrationMode::DockerSwarm {
                assert!(args.contains("stack\ndeploy\n"));
                assert!(args.contains("--with-registry-auth\n"));
            } else {
                assert!(args.contains("compose\n-p\ntest-stack\n"));
                assert!(args.contains("up\n-d\n--pull\nalways\n--remove-orphans\n"));
            }
        }
    }
}

#[tokio::test]
async fn failed_swarm_conversion_restores_compose_without_removing_volumes() {
    let fixture = Fixture::new(
        r#"
root=$(dirname "$0")
printf '%s\n' "$*" >> "$root/calls"
case "$1" in stack) exit 7;; esac
"#,
    );
    let source = stack_source();
    std::fs::write(fixture.0.join("compose.yml"), "services: {}").unwrap();
    let result = stack(&fixture, StackOrchestrationMode::DockerSwarm)
        .apply_directory(
            &fixture.0,
            &source,
            &[],
            None,
            true,
            &CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.status, StackReleaseStatus::Failed);
    let calls = fixture.read("calls");
    let calls: Vec<_> = calls.lines().collect();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].starts_with("compose ") && calls[0].ends_with("down"));
    assert!(calls[1].starts_with("stack deploy "));
    assert!(calls[2].starts_with("compose ") && calls[2].ends_with("up -d --pull missing"));
    assert!(!calls.iter().any(|v| v.contains("--volumes")));
    assert!(!fixture.0.join(".citadel/environment.env").exists());
}

#[tokio::test]
async fn dropping_stack_apply_kills_the_process_and_removes_temporary_source() {
    let fixture = Fixture::new(
        r#"
root=$(dirname "$0")
pwd > "$root/workspace"
printf '%s' "$$" > "$root/pid"
exec sleep 30
"#,
    );
    let runtime = stack(&fixture, StackOrchestrationMode::DockerCompose);
    let source = stack_source();
    let cancel = CancellationToken::new();
    let mut apply = Box::pin(runtime.apply(&source, &[], None, &cancel, None));
    tokio::time::timeout(Duration::from_secs(3), async {
        tokio::select! {
            result = &mut apply => panic!("apply returned early: {result:?}"),
            () = async { while !fixture.0.join("pid").exists() { tokio::time::sleep(Duration::from_millis(10)).await; } } => {}
        }
    }).await.unwrap();
    let pid = fixture.read("pid");
    let workspace = fixture.read("workspace");
    drop(apply);
    assert!(!PathBuf::from(workspace.trim()).exists());
    tokio::time::timeout(Duration::from_secs(3), async {
        while PathBuf::from(format!("/proc/{}", pid.trim())).exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
