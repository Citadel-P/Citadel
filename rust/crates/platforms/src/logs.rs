use crate::RuntimeCapabilityError;
use futures_util::{future::BoxFuture, stream::BoxStream};
use tokio_util::sync::CancellationToken;

pub const MAX_LOG_FRAME: usize = 1024 * 1024;
pub type RuntimeLogStream = BoxStream<'static, Result<Vec<u8>, RuntimeCapabilityError>>;

#[derive(Debug, Clone, Copy)]
pub enum LogResource<'a> {
    Container(&'a str),
    Service(&'a str),
}

#[derive(Debug, serde::Serialize)]
pub struct LogSnapshot {
    pub lines: Vec<String>,
    pub truncated: bool,
}

pub trait LogReadPort: Send + Sync {
    fn read_logs<'a>(
        &'a self,
        resource: LogResource<'a>,
        tail: u16,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<LogSnapshot, RuntimeCapabilityError>>;
}

pub trait ContainerLogPort: Send + Sync {
    fn container_logs<'a>(
        &'a self,
        container_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeLogStream, RuntimeCapabilityError>>;
}
