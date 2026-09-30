//! Server-owned OpenAPI descriptions of git values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

enum_schema!(
    GitRepositoryStatusSchema,
    "GitRepositoryStatus",
    citadel_git::GitRepositoryStatus,
    [Unknown, Pending, Created, Healthy, Degraded]
);

enum_schema!(
    GitRepositoryRefStatusSchema,
    "GitRepositoryRefStatus",
    citadel_git::GitRepositoryRefStatus,
    [Pending, Syncing, Healthy, Degraded]
);
