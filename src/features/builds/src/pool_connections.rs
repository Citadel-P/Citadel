use crate::BuildError;
use futures_util::future::BoxFuture;
pub trait BuildPoolConnections: Send + Sync {
    fn expire_connections(&self) -> BoxFuture<'_, Result<(), BuildError>>;
}
