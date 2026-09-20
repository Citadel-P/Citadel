pub use self::webhooks::WebhookConfiguration as GitRepositoryWebhook;
use self::webhooks::{WebhookError, repository_matches, webhook_branch};
use crate::GitAccountService;
use crate::GitAuthConfiguration;
use crate::GitChangedPath;
use crate::GitCli;
use crate::GitEntryType;
use crate::GitError;
use crate::GitTransport;
use crate::GitTreeEntry;
use crate::RemoteBranch;
use crate::SyncResult;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
mod execution;
pub use execution::GitRepositoryExecutionService;
mod read_models;
pub use read_models::{
    GitBrowserEntryType, GitCommitComparison, GitComposeDiscovery, GitComposeProjectCandidate,
    GitDirectoryEntry, GitDirectoryListing, GitFileContent, GitRepositoryRef, GitSnapshot,
    GitSnapshotFile,
};
mod model;
pub use model::{GitRepositorySource, GitRepositorySyncMode, GitSyncClaim, GitWebhookOutcome};
mod repository;
pub use repository::GitRepositoryExecutionPersistence;
mod error;
pub use error::GitRepositoryExecutionError;

mod commands;
pub mod webhooks;
pub use commands::{
    CreateGitRepository, FieldPatch, GitRepositoryMutationKind, GitRepositoryPatch,
};
pub use model::{GitRepository, GitRepositoryTag, RepoCommand};
pub use repository::GitRepositoryPersistence;
mod service;
pub use error::GitRepositoryError;
pub use service::GitRepositoryService;

pub use commands::{validate_description, validate_name_identifier};
