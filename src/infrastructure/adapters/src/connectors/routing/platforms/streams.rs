//! Stream capabilities share runtime selection and keep transport dispatch internal.
use super::runtime::{PlatformRuntimeRouter, Runtime};
use citadel_platforms::{RuntimeCapabilityError, logs::*, terminal::*};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

pub(super) struct RoutedContainerStreams {
    pub router: PlatformRuntimeRouter,
    pub platform: uuid::Uuid,
    pub node: Option<String>,
}

impl ContainerLogPort for RoutedContainerStreams {
    fn container_logs<'a>(
        &'a self,
        container: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeLogStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            match self
                .router
                .resolve(self.platform, self.node.as_deref(), false, cancel)
                .await?
            {
                Runtime::Local(runtime) => runtime.container_logs(container, cancel).await,
                Runtime::Agent(runtime) => runtime.container_logs(container, cancel).await,
                Runtime::Edge(runtime) => runtime.container_logs(container, cancel).await,
            }
        })
    }
}

impl ContainerTerminalPort for RoutedContainerStreams {
    fn container_terminal<'a>(
        &'a self,
        container: &'a str,
        shell: TerminalShell,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<TerminalSession, RuntimeCapabilityError>> {
        Box::pin(async move {
            match self
                .router
                .resolve(self.platform, self.node.as_deref(), false, cancel)
                .await?
            {
                Runtime::Local(runtime) => {
                    runtime.container_terminal(container, shell, cancel).await
                }
                Runtime::Agent(runtime) => {
                    runtime.container_terminal(container, shell, cancel).await
                }
                Runtime::Edge(runtime) => {
                    runtime.container_terminal(container, shell, cancel).await
                }
            }
        })
    }
}
