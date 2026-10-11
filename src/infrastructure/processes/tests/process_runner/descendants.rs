use super::*;

enum Stop {
    Timeout,
    Cancel,
    Abort,
}

async fn check_descendant_cleanup(stop: Stop) {
    // Cover both a live parent and one that exited leaving inherited pipes.
    for mode in ["descendant-wait", "descendant-exit"] {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
        let cancellation = CancellationToken::new();
        let token = cancellation.clone();
        let task = tokio::spawn(async move {
            run(
                helper(
                    mode,
                    ProcessLimits {
                        timeout: Duration::from_secs(2),
                        ..ProcessLimits::default()
                    },
                )
                .output(sender),
                &token,
            )
            .await
        });
        let pid = match tokio::time::timeout(Duration::from_secs(5), child_pid(&mut receiver)).await
        {
            Ok(pid) => pid,
            Err(error) => {
                task.abort();
                panic!("descendant did not start: {error}");
            }
        };
        match stop {
            Stop::Cancel => cancellation.cancel(),
            Stop::Abort => task.abort(),
            Stop::Timeout => {}
        }
        let result = task.await;
        // Check cleanup before asserting the return value so a failing test
        // also terminates its bounded (20 second) fixture process.
        let stopped = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
                if stat.is_err() || stat.as_ref().is_ok_and(|stat| stat.contains(") Z ")) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        if stopped.is_err() {
            let pid = rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap();
            let _ = rustix::process::kill_process(pid, rustix::process::Signal::KILL);
        }
        stopped.expect("cleanup must stop descendants; init reaps orphaned grandchildren");
        match stop {
            Stop::Timeout => assert!(matches!(result.unwrap(), Err(ProcessError::Timeout(_)))),
            Stop::Cancel => assert!(matches!(result.unwrap(), Err(ProcessError::Cancelled))),
            Stop::Abort => assert!(result.unwrap_err().is_cancelled()),
        }
    }
}

#[tokio::test]
#[ignore = "manual Linux process-group regression; run with --include-ignored"]
async fn timeout_stops_descendants_even_after_the_parent_exits() {
    check_descendant_cleanup(Stop::Timeout).await;
}

#[tokio::test]
#[ignore = "manual Linux process-group regression; run with --include-ignored"]
async fn cancellation_stops_descendants_even_after_the_parent_exits() {
    check_descendant_cleanup(Stop::Cancel).await;
}

#[tokio::test]
#[ignore = "manual Linux process-group regression; run with --include-ignored"]
async fn dropping_the_runner_stops_descendants_even_after_the_parent_exits() {
    check_descendant_cleanup(Stop::Abort).await;
}

#[tokio::test]
#[ignore = "manual Linux process-group regression; run with --include-ignored"]
async fn cancelling_one_command_does_not_stop_another_commands_helpers() {
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
        let cancellation = CancellationToken::new();
        let token = cancellation.clone();
        let task = tokio::spawn(async move {
            run(
                helper("descendant-wait", ProcessLimits::default()).output(sender),
                &token,
            )
            .await
        });
        let pid = child_pid(&mut receiver).await;
        tasks.push((cancellation, task, pid));
    }
    let (first_stop, first, _) = tasks.remove(0);
    let (second_stop, second, second_pid) = tasks.remove(0);
    first_stop.cancel();
    let first_result = first.await;
    let second_is_running = std::fs::read_to_string(format!("/proc/{second_pid}/stat"))
        .is_ok_and(|stat| !stat.contains(") Z "));
    second_stop.cancel();
    let second_result = second.await;
    assert!(
        second_is_running,
        "cleanup must target only the owned group"
    );
    assert!(matches!(
        first_result.unwrap(),
        Err(ProcessError::Cancelled)
    ));
    assert!(matches!(
        second_result.unwrap(),
        Err(ProcessError::Cancelled)
    ));
}
