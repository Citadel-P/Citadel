use super::*;
use std::os::unix::fs::PermissionsExt;

/// Exercise the real executor against Docker. The wrapper injects a competing
/// creator at each vulnerable boundary; it does not simulate restore results.
pub(super) async fn reject_raced_destinations(
    pool: &sqlx::PgPool,
    store: &PostgresBackupPersistence,
    original: &RestoreClaim,
) {
    for stage in [
        "allocate",
        "attach",
        "consumer",
        "start",
        "lost-response",
        "invalid-response",
        "cancel-create",
        "foreign-helper",
    ] {
        let target = format!("citadel-backup-race-{}", Uuid::now_v7().simple());
        let script = std::env::temp_dir().join(&target);
        std::fs::create_dir(&script).unwrap();
        let result = AssertUnwindSafe(async {
            let path = script.join("docker-race");
            // The target is a generated UUID, never user-controlled shell text.
            let injection = match stage {
                "allocate" => format!(
                    "if [ \"$1 $2\" = 'volume create' ]; then\n{FOREIGN_VOLUME}\nfi"
                ),
                "attach" => format!(
                    "if [ \"$1\" = create ]; then\ndocker volume rm '{target}' >/dev/null\n{FOREIGN_VOLUME}\nfi"
                ),
                "start" => "if [ \"$1\" = start ]; then echo 'injected start failure' >&2; exit 42; fi".into(),
                "consumer" => format!(
                    "if [ \"$1\" = create ]; then\n{FOREIGN_VOLUME}\ndocker create --name '$TARGET-consumer' --volume '$TARGET:/data' '$IMAGE' version >/dev/null\nfi"
                ),
                "lost-response" => "if [ \"$1\" = create ]; then docker \"$@\" >/dev/null; echo 'injected response loss' >&2; exit 42; fi".into(),
                "invalid-response" => "if [ \"$1\" = create ]; then docker \"$@\" >/dev/null; echo 'invalid-id'; exit 0; fi".into(),
                "cancel-create" => format!("if [ \"$1\" = create ]; then docker \"$@\" >/dev/null; touch '{}'; sleep 60; exit 42; fi", script.join("created").display()),
                "foreign-helper" => format!(
                    "if [ \"$1\" = create ]; then\nprevious=''\nfor arg in \"$@\"; do\nif [ \"$previous\" = --name ]; then name=\"$arg\"; break; fi\nprevious=\"$arg\"\ndone\ndocker create --name \"$name\" --label com.citadel.system-role=backup-helper '$IMAGE' version > '{}'\nfi", script.join("foreign-id").display()
                ),
                _ => unreachable!(),
            }.replace("$TARGET", &target).replace("$IMAGE", IMAGE);
            std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{injection}\nexec docker \"$@\"\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let queued = store.enqueue_restore(BackupRestoreRequest {
                actor: ActorId::new(SYSTEM_ACTOR_ID), backup_run_id: original.source.id,
                target_platform_id: original.run.target_platform_id,
                target_volume_name: target.clone(), target_host_path: None,
                overwrite_existing: false, target_docker_node_id: None,
                source_backup_run_item_id: None,
            }).await.unwrap();
            let claim = store.claim_restore(Utc::now() - chrono::Duration::hours(1)).await.unwrap().unwrap();
            assert_eq!(claim.run.id, queued.id);
            let executor = DockerResticBackupExecutor::new(
                path, IMAGE, Arc::new(Password), 256 * 1024, pool.clone(),
            );
            let cancellation = CancellationToken::new();
            let restore = executor.restore(&claim, &cancellation);
            let interrupt = async {
                if stage == "cancel-create" {
                    tokio::time::timeout(Duration::from_secs(30), async {
                        while !script.join("created").exists() {
                            tokio::time::sleep(Duration::from_millis(20)).await;
                        }
                    }).await.expect("Helper creation did not reach the injected interruption");
                    cancellation.cancel();
                }
            };
            let (result, ()) = tokio::join!(restore, interrupt);
            store.finish_restore(&claim, &result).await.unwrap();
            let status = if stage == "cancel-create" { BackupRestoreStatus::Cancelled } else { BackupRestoreStatus::Failed };
            assert_eq!(result.status, status, "{stage}: {result:?}");
            let message = result.error_message.unwrap();
            let expected = match stage {
                "start" => "injected start failure",
                "consumer" => "in use by another container",
                "lost-response" => "injected response loss",
                "invalid-response" => "no valid container identity",
                "cancel-create" => "cancel",
                "foreign-helper" => "cleanup could not be verified",
                _ => "ownership could not be verified",
            };
            assert!(message.contains(expected), "{stage}: {message}");
            // Inspect before mounting: docker run would otherwise auto-create a
            // wrongly deleted volume and conceal destructive failure cleanup.
            docker(&["volume", "inspect", &target], None).await;
            if matches!(stage, "allocate" | "attach" | "consumer") {
                let bytes = docker(&["run", "--rm", "--volume", &format!("{target}:/fixture:ro"), "--entrypoint", "sh", IMAGE, "-ec", "test ! -e /fixture/payload.txt; cat /fixture/sentinel"], None).await;
                assert_eq!(bytes, b"unrelated-data");
            }
            let helpers = docker(&["ps", "--all", "--quiet", "--filter", &format!("name=citadel-restore-{}", queued.id.simple())], None).await;
            if stage == "foreign-helper" {
                let foreign = std::fs::read_to_string(script.join("foreign-id")).unwrap();
                let inspected = docker(&["container", "inspect", "--format", "{{.Id}}", foreign.trim()], None).await;
                assert_eq!(String::from_utf8(inspected).unwrap().trim(), foreign.trim());
                assert!(!helpers.is_empty(), "A same-name foreign helper must be preserved");
            } else {
                assert!(helpers.is_empty(), "Failed restore must remove its helper: {stage}");
            }
            if matches!(stage, "lost-response" | "invalid-response" | "cancel-create") {
                assert!(!message.contains("cleanup could not be verified"), "{message}");
            }
        }).catch_unwind().await;
        if let Ok(id) = std::fs::read_to_string(script.join("foreign-id")) {
            let _ = run(
                ProcessRequest::new("docker").args(["rm", "--force", id.trim()]),
                &CancellationToken::new(),
            )
            .await;
        }
        if stage == "consumer" {
            let _ = run(
                ProcessRequest::new("docker").args([
                    "rm",
                    "--force",
                    &format!("{target}-consumer"),
                ]),
                &CancellationToken::new(),
            )
            .await;
        }
        let _ = run(
            ProcessRequest::new("docker").args(["volume", "rm", &target]),
            &CancellationToken::new(),
        )
        .await;
        std::fs::remove_dir_all(&script).unwrap();
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }
}

const FOREIGN_VOLUME: &str = "docker volume create '$TARGET' >/dev/null\ndocker run --rm --volume '$TARGET:/fixture' --entrypoint sh '$IMAGE' -ec 'printf unrelated-data > /fixture/sentinel' >/dev/null";
