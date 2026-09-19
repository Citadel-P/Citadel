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
    SystemGitProcess,
};
pub use repositories::{
    GitBrowserEntryType, GitCommitComparison, GitComposeDiscovery, GitComposeProjectCandidate,
    GitDirectoryEntry, GitDirectoryListing, GitFileContent, GitRepositoryExecutionError,
    GitRepositoryExecutionPersistence, GitRepositoryExecutionService, GitRepositoryRef,
    GitRepositorySource, GitRepositorySyncMode, GitRepositoryWebhook, GitSnapshot, GitSnapshotFile,
    GitSyncClaim, GitWebhookOutcome,
};

pub use repositories::{
    CreateGitRepository, GitRepository, GitRepositoryError, GitRepositoryMutationKind,
    GitRepositoryPatch, GitRepositoryPersistence, GitRepositoryService, GitRepositoryTag,
    RepoCommand,
};

pub mod permissions;
