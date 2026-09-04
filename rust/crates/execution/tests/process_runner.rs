use std::time::Duration;

use citadel_execution::{OutputLimitPolicy, ProcessError, ProcessLimits, ProcessRequest, run};
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
        _ => {}
    }
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
