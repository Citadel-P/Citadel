use super::*;
use std::time::Duration;

#[tokio::test]
async fn cancellation_and_drop_stop_hook_children_even_after_the_shell_exits() {
    for abort in [false, true] {
        for parent_exit in [false, true] {
            let path = std::env::temp_dir().join(format!("citadel-stack-child-{}", Uuid::now_v7()));
            let child_path = path.clone();
            let cancel = CancellationToken::new();
            let token = cancel.clone();
            let task = tokio::spawn(async move {
                run_process(
                    "/bin/sh",
                    &[
                        "-c".into(),
                        format!(
                            "sleep 30 & echo $! > \"$1\"; {}",
                            if parent_exit { "exit 0" } else { "wait" }
                        ),
                        "fixture".into(),
                        child_path.to_string_lossy().into_owned(),
                    ],
                    &std::env::temp_dir(),
                    &[],
                    &token,
                    None,
                )
                .await
            });
            let pid: u32 = tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if let Ok(value) = tokio::fs::read_to_string(&path).await
                        && let Ok(pid) = value.trim().parse()
                    {
                        break pid;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            if abort {
                task.abort();
            } else {
                cancel.cancel();
            }
            let result = task.await;
            let stopped = tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
                    if stat.is_err() || stat.as_ref().is_ok_and(|s| s.contains(") Z ")) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await;
            if stopped.is_err() {
                let _ = std::process::Command::new("kill")
                    .args(["-KILL", &pid.to_string()])
                    .status();
            }
            let _ = std::fs::remove_file(path);
            stopped.expect("Stack hook descendant survived cleanup");
            if abort {
                assert!(result.unwrap_err().is_cancelled());
            } else {
                assert!(matches!(result.unwrap(), Err(StackError::Cancelled)));
            }
        }
    }
}
