use crate::*;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

pub trait ScriptRunTokenIssuer: Send + Sync {
    fn issue<'a>(
        &'a self,
        run_as_actor_id: ActorId,
        run_id: Uuid,
        attempt_id: Uuid,
        lifetime: Duration,
    ) -> BoxFuture<'a, Result<String, ScriptError>>;
}

pub trait ScriptEntitlements: Send + Sync {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, ScriptError>>;
}

pub struct ScriptRuntimeConfig {
    pub deno_path: OsString,
    pub work_root: PathBuf,
    pub internal_base_url: String,
    pub endpoint_catalog_json: String,
    pub maximum_log_bytes: usize,
    pub stale_after: Duration,
}
