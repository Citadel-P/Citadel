//! Opt-in ownership scenario, not an RSS benchmark. Docker is a disposable CLI
//! fixture; Deno and the source staging/process/output/cleanup adapters are real.
use super::*;
use citadel_builds::NoopBuildLogSink;
use citadel_docker::{builds::output, stacks::LocalStackApply};
use citadel_execution::{OutputLimitPolicy, ProcessLimits};
use citadel_stacks::{
    StackApplySource, StackOrchestrationMode, StackReleaseStatus, StackSourceFile,
};
use std::os::unix::fs::PermissionsExt;

#[tokio::test]
#[ignore = "requires CITADEL_DENO_PATH; fake Docker CLI and disposable filesystem only"]
async fn repeated_stack_build_and_deno_cycles_release_execution_resources() {
    let tools = tempfile::tempdir().unwrap();
    let executable = tools.path().join("docker-fixture");
    let observed = tools.path().join("staged-root");
    std::fs::write(
        &executable,
        r#"#!/bin/sh
set -eu
if [ "$1" = build ]; then
    (head -c 2097152 /dev/zero | tr '\0' x) &
    (head -c 2097152 /dev/zero | tr '\0' y >&2) &
    wait
else
    test -s compose.yml
    if [ "${FIXTURE_GIT:-0}" = 1 ]; then
        test -s app/Dockerfile
        test -s app/payload
        test -s settings.env
        test -s config/runtime.conf
        test "$(wc -c < app/payload)" -eq 1048576
    fi
    pwd > "$FIXTURE_OBSERVED_ROOT"
    printf 'deployed\n'
fi
"#,
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let fixture = Fixture::new();
    let sandbox = fixture.sandbox(
        Arc::new(citadel_processes::SystemProcess),
        std::env::var_os("CITADEL_DENO_PATH").expect("CITADEL_DENO_PATH is required"),
    );
    let cancellation = CancellationToken::new();
    // Cycle zero warms the same paths exercised by subsequent cycles.
    for cycle in 0..4 {
        eprintln!(
            "cycle={cycle} phase={}",
            if cycle == 0 { "warmup" } else { "baseline" }
        );
        for git in [false, true] {
            let mut source = StackApplySource {
                files: vec![StackSourceFile {
                    relative_path: "compose.yml".into(),
                    content: b"services:\n  app:\n    image: fixture\n".to_vec(),
                }],
                compose_paths: vec!["compose.yml".into()],
                env_file_paths: vec![],
                working_directory: ".".into(),
                labels_override_path: None,
                resolved_commit_sha: git.then(|| "a".repeat(40)),
            };
            if git {
                for (path, bytes) in [
                    ("app/Dockerfile", b"FROM scratch\n".to_vec()),
                    ("app/payload", vec![b'x'; 1024 * 1024]),
                    ("settings.env", b"SETTING=value\n".to_vec()),
                    ("config/runtime.conf", b"enabled=true\n".to_vec()),
                ] {
                    source.files.push(StackSourceFile {
                        relative_path: path.into(),
                        content: bytes,
                    });
                }
                source.env_file_paths.push("settings.env".into());
            }
            let runtime = LocalStackApply {
                docker: executable.to_str().unwrap().into(),
                endpoint: "unix:///fixture-no-daemon.sock".parse().unwrap(),
                project_name: format!("fixture-{cycle}"),
                orchestration: StackOrchestrationMode::DockerCompose,
                destroy_before_deploy: false,
                pre_deploy: None,
                post_deploy: None,
                service_names: vec![],
                pull_images: false,
            };
            let result = runtime
                .apply(
                    &source,
                    &[
                        format!("FIXTURE_GIT={}", usize::from(git)),
                        format!("FIXTURE_OBSERVED_ROOT={}", observed.display()),
                    ],
                    None,
                    &cancellation,
                    None,
                )
                .await
                .unwrap();
            assert_eq!(result.status, StackReleaseStatus::Healthy);
            let root = std::fs::read_to_string(&observed).unwrap();
            assert!(
                !std::path::Path::new(root.trim()).exists(),
                "transient stack source survived completion"
            );
            drop(source);
            drop(result);
        }
        eprintln!("cycle={cycle} phase=stack-cleaned");
        let request = ProcessRequest::new(executable.as_os_str())
            .args(["build"])
            .limits(ProcessLimits {
                timeout: Duration::from_secs(10),
                maximum_stdout_bytes: 1024 * 1024,
                maximum_stderr_bytes: 1024 * 1024,
                output_limit_policy: OutputLimitPolicy::Truncate,
            });
        let output = output::run(request, &cancellation, &NoopBuildLogSink, &[])
            .await
            .unwrap();
        assert!(output.succeeded());
        assert_eq!(output.stdout.capacity() + output.stderr.capacity(), 0);
        assert!(output.stdout_truncated && output.stderr_truncated);
        eprintln!("cycle={cycle} phase=build-cleaned retained-transcript-bytes=0");
        let result = sandbox.execute(input("const p = Deno.env.get('DENO_DIR'); await Deno.mkdir(p, {recursive:true}); await Deno.writeTextFile(p + '/probe', 'cache'); console.log('done');"), &cancellation, None).await;
        assert_eq!(result.status, ScriptRunStatus::Succeeded);
        fixture.assert_empty();
        assert_eq!(fixture.tasks.active(), 0);
        eprintln!("cycle={cycle} phase=deno-cleaned tracked-cleanups=0 run-directories=0");
    }
    fixture.tasks.drain(Duration::from_secs(5)).await.unwrap();
}
