use super::tests::{Fixture, resolved};
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest, ProcessRunner};
use futures_util::future::BoxFuture;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_util::sync::CancellationToken;

struct Resolver {
    calls: AtomicUsize,
    change: &'static str,
    input: std::path::PathBuf,
}
impl ProcessRunner for Resolver {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            assert_eq!(request.program, "docker");
            assert!(request.arguments.ends_with(&[
                "config".into(),
                "--format".into(),
                "json".into()
            ]));
            let number = self.calls.fetch_add(1, Ordering::SeqCst);
            let snapshot = request.current_directory.as_ref().unwrap();
            let env = std::fs::read_to_string(snapshot.join(".env")).unwrap();
            let mut value = resolved();
            if number == 1 {
                let image = env
                    .lines()
                    .find_map(|line| line.strip_prefix("CITADEL_IMAGE="))
                    .unwrap();
                value["services"]["server"]["image"] = json!(image);
                match self.change {
                    "database" => value["services"]["pg_db"]["image"] = json!("postgres:18.7"),
                    "credentials" => {
                        value["services"]["server"]["environment"]["PG_PASSWORD"] =
                            json!("changed");
                        value["services"]["pg_db"]["environment"]["POSTGRES_PASSWORD"] =
                            json!("changed");
                    }
                    "port" => {
                        value["services"]["server"]["ports"] =
                            json!([{"target":8080,"published":"9999","protocol":"tcp"}])
                    }
                    "ignored-image" => {
                        value["services"]["server"]["image"] =
                            json!("ghcr.io/citadel-p/citadel:latest")
                    }
                    "volume" => value["volumes"]["citadel_data"]["name"] = json!("different_data"),
                    "edit-input" => std::fs::write(&self.input, "PG_PASSWORD=changed\n").unwrap(),
                    "cancel" => cancellation.cancel(),
                    "normal" => (),
                    _ => panic!("unknown fixture"),
                }
            }
            Ok(ProcessOutput {
                exit_code: Some(0),
                stdout: serde_json::to_vec(&value).unwrap(),
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            })
        })
    }
}

#[tokio::test]
async fn replacement_preserves_inputs_and_rejects_other_resolved_changes() {
    let image = format!("ghcr.io/citadel-p/citadel@sha256:{}", "e".repeat(64));
    for change in [
        "normal",
        "database",
        "credentials",
        "port",
        "volume",
        "ignored-image",
        "edit-input",
        "cancel",
    ] {
        let fixture = Fixture::new(true);
        let path = fixture.inputs.directory.join(".env");
        let before = std::fs::read(&path).unwrap();
        let runner = Resolver {
            calls: AtomicUsize::new(0),
            change,
            input: path.clone(),
        };
        let result = fixture
            .inputs
            .check_replacement(&image, &runner, &CancellationToken::new())
            .await;
        assert_eq!(result.is_ok(), change == "normal", "{change}: {result:?}");
        assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
        if change != "edit-input" {
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
        if let Ok(receipt) = result {
            assert_eq!(receipt.target_image, image);
            assert_ne!(
                receipt.target_configuration_sha256,
                receipt.source_configuration_sha256
            );
            assert!(
                !serde_json::to_string(&receipt)
                    .unwrap()
                    .contains("private-fixture-password")
            );
        }
    }
}

#[tokio::test]
async fn replacement_rejects_unpinned_images_and_precancellation_without_processes() {
    let fixture = Fixture::new(false);
    let runner = Resolver {
        calls: AtomicUsize::new(0),
        change: "normal",
        input: fixture.inputs.directory.join(".env"),
    };
    for image in ["ghcr.io/citadel-p/citadel:latest", "other@sha256:abc", ""] {
        assert!(
            fixture
                .inputs
                .check_replacement(image, &runner, &CancellationToken::new())
                .await
                .is_err()
        );
    }
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert!(
        fixture
            .inputs
            .check_replacement(
                &format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64)),
                &runner,
                &cancellation
            )
            .await
            .is_err()
    );
    assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[ignore = "Requires the packaged Docker Compose CLI; reads disposable local inputs only"]
async fn actual_compose_replacement_preserves_the_installer_and_edge_overlay() {
    let process = citadel_processes::SystemProcess;
    let image = format!("ghcr.io/citadel-p/citadel@sha256:{}", "a".repeat(64));
    for edge in [false, true] {
        let fixture = Fixture::new(edge);
        let before = fixture.inputs.capture().unwrap();
        let checked = fixture
            .inputs
            .check_replacement(&image, &process, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(checked.target_image, image);
        assert_eq!(fixture.inputs.capture().unwrap(), before);
    }
}

#[tokio::test]
async fn publishes_only_reviewed_environment_and_refuses_replay_or_changed_inputs() {
    use std::os::unix::fs::{MetadataExt, symlink};
    for mode in [
        "normal",
        "drift",
        "cancel",
        "symlink",
        "hardlink",
        "target-hash",
    ] {
        let f = Fixture::new(true);
        let env = f.inputs.directory.join(".env");
        let original = std::fs::read(&env).unwrap();
        let compose: Vec<_> = f
            .inputs
            .compose_files
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect();
        let runner = Resolver {
            calls: AtomicUsize::new(0),
            change: "normal",
            input: env.clone(),
        };
        let image = format!("ghcr.io/citadel-p/citadel@sha256:{}", "e".repeat(64));
        let token = CancellationToken::new();
        let mut candidate = f
            .inputs
            .prepare_replacement(&image, &runner, &token)
            .await
            .unwrap();
        let checked = candidate.checked.clone();
        match mode {
            "drift" => std::fs::write(&env, "PG_PASSWORD=concurrent-edit\n").unwrap(),
            "cancel" => token.cancel(),
            "symlink" => {
                std::fs::rename(&env, f.inputs.directory.join("saved-env")).unwrap();
                symlink("saved-env", &env).unwrap();
            }
            "hardlink" => std::fs::hard_link(&env, f.inputs.directory.join("saved-env")).unwrap(),
            "target-hash" => candidate.checked.target_configuration_sha256 = "a".repeat(64),
            _ => (),
        }
        let before = std::fs::read(&env).unwrap();
        assert_eq!(
            candidate.publish(&f.inputs, &token).is_ok(),
            mode == "normal",
            "{mode}"
        );
        if mode == "normal" {
            let expected = format!(
                "{}CITADEL_IMAGE={image}\n",
                std::str::from_utf8(&original).unwrap()
            );
            assert_eq!(std::fs::read(&env).unwrap(), expected.as_bytes());
            assert_eq!(std::fs::metadata(&env).unwrap().mode() & 0o777, 0o600);
            assert_eq!(
                super::super::registration::fingerprint_inputs(
                    &f.inputs.project,
                    &f.inputs.capture().unwrap()
                ),
                checked.target_configuration_sha256
            );
            let stale = super::publication::PreparedCoreReplacement {
                checked,
                environment: zeroize::Zeroizing::new(expected.into_bytes()),
            };
            assert!(
                stale.publish(&f.inputs, &token).is_err(),
                "original intent cannot publish twice"
            );
        } else {
            assert_eq!(std::fs::read(&env).unwrap(), before);
        }
        for (path, before) in f.inputs.compose_files.iter().zip(compose) {
            assert_eq!(std::fs::read(path).unwrap(), before);
        }
        assert!(
            !std::fs::read_dir(&f.inputs.directory)
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".citadel-image-"))
        );
    }
}

#[tokio::test]
#[ignore = "Requires Docker Compose CLI; edits only private disposable installer files, no daemon"]
async fn actual_compose_accepts_published_image_with_base_and_edge_inputs() {
    let process = citadel_processes::SystemProcess;
    let token = CancellationToken::new();
    let image = format!("ghcr.io/citadel-p/citadel@sha256:{}", "e".repeat(64));
    for edge in [false, true] {
        let f = Fixture::new(edge);
        let before = f.inputs.capture().unwrap();
        let candidate = f
            .inputs
            .prepare_replacement(&image, &process, &token)
            .await
            .unwrap();
        let fingerprint = candidate.checked.target_configuration_sha256.clone();
        candidate.publish(&f.inputs, &token).unwrap();
        let actual = f.inputs.check(&process, &token).await.unwrap();
        assert_eq!(actual.core_image, image);
        assert_eq!(actual.configuration_sha256, fingerprint);
        let after = f.inputs.capture().unwrap();
        assert_eq!(&before[..before.len() - 1], &after[..after.len() - 1]);
    }
}

struct DatabaseResolver {
    change_password: bool,
}
impl ProcessRunner for DatabaseResolver {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            let env =
                std::fs::read_to_string(request.current_directory.as_ref().unwrap().join(".env"))
                    .unwrap();
            let mut value = resolved();
            let target = env.contains("PG_DATA=/var/lib/postgresql/18/docker");
            value["services"]["pg_db"]["environment"]["PGDATA"] =
                json!("/var/lib/postgresql/18/docker");
            if target {
                let image = env
                    .lines()
                    .find_map(|line| line.strip_prefix("PG_IMAGE="))
                    .unwrap();
                value["services"]["pg_db"]["image"] = json!(image);
                value["services"]["server"]["environment"]["PG_IMAGE"] = json!(image);
                value["services"]["server"]["environment"]["PG_DATA"] =
                    json!("/var/lib/postgresql/18/docker");
                if self.change_password {
                    value["services"]["server"]["environment"]["PG_PASSWORD"] = json!("changed");
                    value["services"]["pg_db"]["environment"]["POSTGRES_PASSWORD"] =
                        json!("changed");
                }
            }
            Ok(ProcessOutput {
                exit_code: Some(0),
                stdout: serde_json::to_vec(&value).unwrap(),
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            })
        })
    }
}

#[tokio::test]
async fn database_replacement_changes_only_engine_selectors_and_preserves_core() {
    for change_password in [false, true] {
        let fixture = Fixture::new(true);
        let path = fixture.inputs.directory.join(".env");
        let before = std::fs::read(&path).unwrap();
        let plan = super::database::database_update(180001).unwrap().unwrap();
        let candidate = fixture
            .inputs
            .prepare_database_replacement(
                &plan,
                &DatabaseResolver { change_password },
                &CancellationToken::new(),
            )
            .await;
        assert_eq!(candidate.is_ok(), !change_password);
        assert_eq!(std::fs::read(path).unwrap(), before);
        if let Ok(candidate) = candidate {
            assert_eq!(
                candidate.checked.target_image,
                resolved()["services"]["server"]["image"].as_str().unwrap()
            );
            assert_ne!(
                candidate.checked.source_configuration_sha256,
                candidate.checked.target_configuration_sha256
            );
        }
    }
}
