use crate::*;
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

pub trait AutomationRunTokenIssuer: Send + Sync {
    fn issue<'a>(
        &'a self,
        run_as_actor_id: ActorId,
        run_id: Uuid,
        lifetime: Duration,
    ) -> BoxFuture<'a, Result<String, AutomationError>>;
}

pub trait AutomationEntitlements: Send + Sync {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, AutomationError>>;
}

pub struct AutomationRuntimeConfig {
    pub deno_path: OsString,
    pub work_root: PathBuf,
    pub internal_base_url: String,
    pub endpoint_catalog_json: String,
    pub maximum_log_bytes: usize,
    pub stale_after: Duration,
}
