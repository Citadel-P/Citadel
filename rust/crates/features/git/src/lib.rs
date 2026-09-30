#![forbid(unsafe_code)]
pub mod accounts;
mod cli;
pub mod repositories;
pub use accounts::{
    CreateGitAccount, GitAccount, GitAccountConfiguration, GitAccountError, GitAccountRepository,
    GitAccountService, GitAuthConfiguration, GitAuthType, GitCredentialProtector, GitTransport,
    StoredGitAccount, UpdateGitAccount,
};
pub use cli::{
    GitBlob, GitChangedPath, GitChangedPathListing, GitChangedPathStatus, GitCli, GitEntryType,
    GitError, GitProcessPort, GitTreeEntry, GitTreeListing, RemoteBranch, SyncResult,
};
pub use repositories::{
    GitBrowserEntryType, GitCommitComparison, GitComposeDiscovery, GitComposeProjectCandidate,
    GitDirectoryEntry, GitDirectoryListing, GitFileContent, GitRepositoryExecutionError,
    GitRepositoryExecutionPersistence, GitRepositoryExecutionService, GitRepositoryRef,
    GitRepositorySource, GitRepositorySyncMode, GitSnapshot, GitSnapshotFile, GitSyncClaim,
    GitWebhookOutcome,
};

pub use repositories::{
    CreateGitRepository, GitRepository, GitRepositoryError, GitRepositoryMutationKind,
    GitRepositoryPatch, GitRepositoryPersistence, GitRepositoryService, RepoCommand,
};

pub mod permissions;

mod status;
pub use status::{GitRepositoryRefStatus, GitRepositoryStatus};
