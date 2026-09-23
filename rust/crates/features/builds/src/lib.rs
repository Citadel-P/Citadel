#![forbid(unsafe_code)]

mod jobs;
pub use agent_pools::runtime::{BuildPoolCheck, BuildPoolChecker, BuildPoolTarget, pool_target};
pub use jobs::completion::{
    BuildCompletionRepository, BuildCompletionService, BuildConsumerClaim, BuildConsumerRuntime,
    BuildConsumerType,
};
pub use runs::logs::{BuildLogEntry, BuildLogNotifier, BuildLogSink, NoopBuildLogSink};
pub mod agent_pools;
mod error;
pub mod projects;
mod repository;
pub mod runs;
mod runtime;
mod service;
#[cfg(test)]
mod tests;
mod validation;
pub use agent_pools::{BuildAgentPool, BuildAgentPoolConfiguration};
use chrono::{DateTime, Utc};
use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_primitives::ActorId;
pub use error::BuildError;
use futures_util::future::BoxFuture;
pub use projects::{BuildArgSpec, BuildProject, BuildProjectConfiguration, BuildSecretSpec};
pub use repository::BuildRepository;
pub use runs::{BuildClaim, BuildExecutionResult, BuildLog, BuildPlatformSnapshot, BuildRun};
pub use runtime::{
    BuildEntitlements, BuildExecutor, BuildRegistryCredentialResolver, BuildRegistryCredentials,
    BuildSecretResolver,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use service::BuildService;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use validation::{normalize_path, normalize_required, valid_git_branch, validate_range};

mod tasks;
pub use tasks::BuildTaskSpawner;

pub mod permissions;
