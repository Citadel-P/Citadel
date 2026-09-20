use citadel_alerts::{AlertChannel, AlertDelivery, AlertError, AlertEvent};
use citadel_execution::{OutputLimitPolicy, ProcessLimits, ProcessRequest};
use citadel_processes::run;
use futures_util::future::BoxFuture;
use std::ffi::OsString;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct ShoutrrrAlertDelivery {
    executable: OsString,
    timeout: Duration,
}

impl ShoutrrrAlertDelivery {
    pub fn new(executable: impl Into<OsString>, timeout: Duration) -> Self {
        Self {
            executable: executable.into(),
            timeout,
        }
    }
}

impl AlertDelivery for ShoutrrrAlertDelivery {
    fn send<'a>(
        &'a self,
        channel: &'a AlertChannel,
        event: &'a AlertEvent,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let output = run(
                ProcessRequest::new(self.executable.clone())
                    .args([
                        "send",
                        "--url",
                        channel.url.as_str(),
                        "--title",
                        &format!("[{}] Citadel alert", event.severity),
                        "--message",
                        event.message.as_str(),
                    ])
                    .limits(ProcessLimits {
                        timeout: self.timeout,
                        maximum_stdout_bytes: 64 * 1024,
                        maximum_stderr_bytes: 64 * 1024,
                        output_limit_policy: OutputLimitPolicy::Truncate,
                    }),
                cancellation,
            )
            .await
            .map_err(|error| AlertError::Delivery(error.to_string()))?;
            if output.succeeded() {
                Ok(())
            } else {
                Err(AlertError::Delivery(format!(
                    "Shoutrrr exited with code {}.",
                    output
                        .exit_code
                        .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
                )))
            }
        })
    }
}
