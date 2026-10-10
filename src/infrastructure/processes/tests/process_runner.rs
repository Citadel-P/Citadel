use std::time::Duration;

use citadel_execution::{
    OutputLimitPolicy, ProcessChunk, ProcessError, ProcessLimits, ProcessRequest,
};
use citadel_processes::run;
use tokio_util::sync::CancellationToken;

fn helper(mode: &str, limits: ProcessLimits) -> ProcessRequest {
    ProcessRequest::new(std::env::current_exe().expect("test executable is available"))
        .args(["--exact", "process_helper", "--nocapture"])
        .env("CITADEL_PROCESS_HELPER", mode)
        .limits(limits)
}

#[test]
fn process_helper() {
    use std::io::Write;

    match std::env::var("CITADEL_PROCESS_HELPER").as_deref() {
        Ok("exit") => {
            std::io::stdout().write_all(b"output").unwrap();
            std::io::stderr().write_all(b"error").unwrap();
            std::process::exit(7);
        }
        Ok("output") => loop {
            std::io::stdout().write_all(b"0123456789").unwrap();
        },
        Ok("finite-output") => {
            std::io::stdout().write_all(&[b'x'; 128]).unwrap();
        }
        Ok("wait") => std::thread::sleep(Duration::from_secs(20)),
        Ok("stream-wait") => {
            writeln!(std::io::stdout(), "child-pid={}", std::process::id()).unwrap();
            std::io::stdout().flush().unwrap();
            std::thread::sleep(Duration::from_secs(20));
        }
        _ => {}
    }
}

#[tokio::test]
async fn delivers_output_before_exit_and_cancellation_reaps_the_child() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let cancellation = CancellationToken::new();
    let execution = run(
        helper("stream-wait", ProcessLimits::default()).output(sender),
        &cancellation,
    );
    let observe = async {
        let pid = child_pid(&mut receiver).await;
        cancellation.cancel();
        while receiver.recv().await.is_some() {}
        pid
    };
    let (result, _pid) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(execution, observe)
    })
    .await
    .unwrap();
    assert!(matches!(result, Err(ProcessError::Cancelled)));
    #[cfg(target_os = "linux")]
    assert!(!std::path::Path::new(&format!("/proc/{_pid}")).exists());
}

async fn child_pid(receiver: &mut tokio::sync::mpsc::Receiver<ProcessChunk>) -> u32 {
    let mut received = Vec::new();
    loop {
        let chunk = receiver.recv().await.expect("child is still running");
        assert!(chunk.bytes.len() <= 8192);
        received.extend(chunk.bytes);
        for line in received.split_inclusive(|byte| *byte == b'\n') {
            if line.ends_with(b"\n")
                && let Some(pid) = line.strip_prefix(b"child-pid=")
            {
                return std::str::from_utf8(pid).unwrap().trim().parse().unwrap();
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn aborting_the_runner_kills_and_eventually_reaps_its_child() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    let task = tokio::spawn(async move {
        run(
            helper("stream-wait", ProcessLimits::default()).output(sender),
            &CancellationToken::new(),
        )
        .await
    });
    let pid = tokio::time::timeout(Duration::from_secs(5), child_pid(&mut receiver))
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(5), async {
        while std::path::Path::new(&format!("/proc/{pid}")).exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Tokio must reap a child whose runner was dropped");
}

#[tokio::test]
async fn a_full_output_channel_does_not_prevent_timeout_cleanup() {
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    let limits = ProcessLimits {
        timeout: Duration::from_millis(200),
        ..ProcessLimits::default()
    };
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        run(
            helper("output", limits).output(sender),
            &CancellationToken::new(),
        ),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(ProcessError::Timeout(_))));
}

#[tokio::test]
async fn timeout_still_applies_when_an_exited_child_has_undelivered_output() {
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    sender
        .send(ProcessChunk {
            stream: "stdout",
            bytes: b"occupied".to_vec(),
        })
        .await
        .unwrap();
    let limits = ProcessLimits {
        timeout: Duration::from_millis(200),
        ..ProcessLimits::default()
    };
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        run(
            helper("exit", limits).output(sender),
            &CancellationToken::new(),
        ),
    )
    .await
    .expect("pipe collection must obey the process deadline");
    assert!(matches!(result, Err(ProcessError::Timeout(_))));
}

#[tokio::test]
async fn cancellation_still_applies_when_an_exited_child_has_undelivered_output() {
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    sender
        .send(ProcessChunk {
            stream: "stdout",
            bytes: b"occupied".to_vec(),
        })
        .await
        .unwrap();
    let cancellation = CancellationToken::new();
    let cancel = async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancellation.cancel();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(
            run(
                helper("exit", ProcessLimits::default()).output(sender),
                &cancellation
            ),
            cancel,
        )
    })
    .await
    .expect("pipe collection must remain cancellable");
    assert!(matches!(result, Err(ProcessError::Cancelled)));
}

#[tokio::test]
async fn precancelled_request_does_not_spawn_an_executable() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = run(
        ProcessRequest::new("citadel-deliberately-nonexistent-executable"),
        &cancellation,
    )
    .await;
    assert!(matches!(result, Err(ProcessError::Cancelled)));
}

#[tokio::test]
async fn captures_stdout_stderr_and_exit_code() {
    let output = run(
        helper("exit", ProcessLimits::default()),
        &CancellationToken::new(),
    )
    .await
    .expect("process should run");

    assert_eq!(output.exit_code, Some(7));
    assert!(String::from_utf8_lossy(&output.stdout).contains("output"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("error"));
    assert!(!output.stdout_truncated);
    assert!(!output.stderr_truncated);
}

#[tokio::test]
async fn kills_process_when_output_limit_is_exceeded() {
    let limits = ProcessLimits {
        timeout: Duration::from_secs(5),
        maximum_stdout_bytes: 32,
        maximum_stderr_bytes: 32,
        output_limit_policy: OutputLimitPolicy::Error,
    };

    let error = run(helper("output", limits), &CancellationToken::new())
        .await
        .expect_err("unbounded output must be rejected");

    assert!(matches!(
        error,
        ProcessError::OutputLimit {
            stream: "stdout",
            limit: 32
        }
    ));
}

#[tokio::test]
async fn truncation_policy_drains_the_child_and_returns_bounded_output() {
    let limits = ProcessLimits {
        timeout: Duration::from_millis(100),
        maximum_stdout_bytes: 32,
        maximum_stderr_bytes: 32,
        output_limit_policy: OutputLimitPolicy::Truncate,
    };

    let output = run(helper("finite-output", limits), &CancellationToken::new())
        .await
        .expect("a finite child may complete with truncated output");

    assert_eq!(output.stdout.len(), 32);
    assert!(output.stdout_truncated);
    assert!(!output.stderr_truncated);
}

#[tokio::test]
async fn kills_process_at_timeout() {
    let limits = ProcessLimits {
        timeout: Duration::from_millis(100),
        ..ProcessLimits::default()
    };

    let error = run(helper("wait", limits), &CancellationToken::new())
        .await
        .expect_err("long process must time out");

    assert!(matches!(error, ProcessError::Timeout(_)));
}

#[tokio::test]
async fn kills_process_when_request_is_cancelled() {
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancel.cancel();
    });

    let error = run(helper("wait", ProcessLimits::default()), &cancellation)
        .await
        .expect_err("cancelled process must stop");

    assert!(matches!(error, ProcessError::Cancelled));
}
