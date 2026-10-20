use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, terminal::*};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Cancelled,
        "Terminal session canceled.",
        false,
    )
}

impl ContainerTerminalPort for crate::client::DockerClient {
    fn container_terminal<'a>(
        &'a self,
        id: &'a str,
        shell: TerminalShell,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<TerminalSession, RuntimeCapabilityError>> {
        Box::pin(async move {
            tokio::select! {
                biased;
                ()=cancel.cancelled()=>Err(cancelled()),
                result=tokio::time::timeout(self.request_timeout(),self.open_terminal(id,shell,cancel))=>
                    result.map_err(|_|RuntimeCapabilityError::new(RuntimeErrorKind::Timeout,"Opening terminal timed out.",false))?.map_err(crate::client::runtime::normalize_docker_error),
            }
        })
    }
}
