# Git Repository And GitOps Stack Integration

## Purpose

This spec describes the current Git repository implementation and the intended path toward GitOps-style stack integration.

The goal is to let users define reusable Git repositories, keep a local synchronized cache, and link stacks to repository paths so stack releases can be applied from version-controlled compose files.

Webhook ingestion is out of scope for the first implementation pass. The repository model still records whether sync is manual, interval-polled, or webhook-driven so the UI and persistence shape do not need to be redesigned later.

## Main Files

- `src/Citadel.Domain/Entities/Git/GitAccount.cs`
- `src/Citadel.Domain/Entities/Git/GitRepository.cs`
- `src/Citadel.Domain/Entities/Git/GitRepositoryEntity.cs`
- `src/Citadel.Domain/Entities/Stacks/StackSpec.cs`
- `src/Citadel.Application/Features.GitRepositories/Commands/CreateGitRepository.cs`
- `src/Citadel.Application/Features.GitRepositories/Commands/PatchGitRepository.cs`
- `src/Citadel.Application/TaskJobs/GitRepoSyncJob.cs`
- `src/Citadel.Application/Services/RepoCacheManager.cs`
- `src/Citadel.Application/Services/ApplyStackService.cs`
- `src/Citadel.FrontEnd/src/features/git-repos/form/form.tsx`
- `src/Citadel.FrontEnd/src/features/git-repos/table.tsx`
- `src/Citadel.FrontEnd/src/features/stacks/form/form.tsx`
- `src/Citadel.Infrastructure.Migrations/Migrations/*_migration0003.cs`
- `test/Citadel.Tests.Integration/Application/TaskJobs/GitRepoSyncJobTests.cs`
- `test/Citadel.Tests.Unit/Domain/Entities/Git/GitRepositoryTests.cs`

## Current Model

`GitAccount` stores reusable credentials for a Git provider or host:

- `Name`
- `Domain`
- `Transport`
- `AuthType`
- polymorphic `GitAuthConfiguration`

Supported auth configuration types:

- `TokenAuth`
- `BasicAuth`
- `SshKeyAuth`

`GitAccount` validates that transport and auth configuration are compatible. SSH transport requires SSH key auth, and HTTP/HTTPS cannot use SSH key auth.

`GitRepository` stores a configured repository source:

- `Name`
- `Description`
- `Url`
- `DefaultBranch`
- optional `GitAccountId`
- `SyncMode`
- optional `SyncIntervalMinutes`
- webhook fields
- clone/pull hook commands
- sync status
- processing state

It implements `IReconcilableResource`, so it has the same basic control-state shape as stacks and deployments:

- `ControlState`
- `ControlTriggeredBy`
- `ControlStartedAt`
- `RowVersion`

The repository cache path is currently derived from the repository URL name:

```text
/app/data/repos/{repositoryName}
```

For example, `https://github.com/org/citadel.git` maps to `/app/data/repos/citadel`.

Repository sync modes:

- `Manual`: only syncs when the user clicks Sync or another explicit workflow enqueues a sync.
- `PullInterval`: background polling syncs the default branch and linked stack branches when the branch ref is older than `SyncIntervalMinutes`.
- `Webhook`: stores webhook intent and secret, and disables polling. Actual webhook ingestion is still a later slice.

## Current Repository Sync Flow

Creating a Git repository:

1. validates name, URL, default branch, account existence, and account domain match
2. creates the `GitRepository`
3. marks it processing
4. records `GitRepoCreated`
5. sends a repository notification
6. queues `GitRepoSyncRequest`

`GitRepoSyncJob` handles queued sync requests:

1. loads the repository with its account
2. tests Git connectivity
3. clones or pulls the local cache through `RepoCacheManager`
4. records `GitRepoCloned` or `GitRepoPulled`
5. marks the repository `Healthy` or `Degraded`
6. sends activity and repository notifications

`GitRepositoryPollingJob` enqueues polling sync only for repositories whose sync mode is `PullInterval`. It checks the repository default branch plus linked Git stack branches, then uses `gitrepositoryrefs.LastSyncedAt` and `SyncIntervalMinutes` to avoid unnecessary sync requests.

`RepoCacheManager` serializes sync per repository id with an in-memory semaphore. It clones if the local cache does not have `.git`; otherwise it pulls the configured branch.

`OnClone` hooks run after clone. `OnPull` hooks run after both clone and pull.

## Current Frontend Surface

The Git repository table shows:

- name
- URL
- default branch
- created time
- status and processing indicator
- edit/delete actions

The Git repository form supports:

- name and description on create
- URL
- default branch
- optional Git account
- sync mode: pull on interval, webhook, or manual only
- interval minutes when pull-on-interval is selected
- webhook secret when webhook mode is selected
- `OnClone` command
- `OnPull` command

The stack form already has Git stack fields:

- repository selector
- branch
- optional commit SHA
- compose paths
- additional env files from repository
- update behavior
- common stack environment, registry, hooks, drift policy, project name, and destroy behavior

## Current Stack Integration

`GitStack` already exists in `StackSpec`.

Current shape:

- `GitRepoId`
- `Branch`
- `CommitSha`
- `UpdateBehavior`
- `ProjectName`
- `WebHookEnabled`
- `WebHookForceDeploy`
- `WebHookSecret`
- `ComposePaths`
- `PreDeploy`
- `PostDeploy`
- `EnvVars`
- `AdditionalEnvFileFromRepo`
- `EnvFilePath`
- `RegistryId`
- `DestroyBeforeDeploy`

Stack create and patch validate that the referenced repository exists.

The stack lookup endpoint includes the currently selected Git repository even if normal permissions would otherwise filter it out, matching the existing platform/registry lookup pattern.

The major missing piece is apply support. `ApplyStackService` currently rejects every non-`ManualStack` spec:

```text
Only manual stack specs are currently supported for stack apply.
```

So Git-backed stacks can be configured, but they cannot yet be materialized into compose content and deployed.

## GitOps Semantics

The recommended model is:

- `GitRepository` owns repository connectivity, local cache, sync status, and provider-level metadata.
- `GitStack` owns how a stack consumes a repository: branch, optional pin, compose paths, env file paths, registry, and deployment behavior.
- `StackRelease` owns what was actually applied.

A repository sync should not directly mutate stack release specs. It should detect whether linked Git stacks have newer source content and then update stack update state or trigger apply based on the stack policy.

GitOps should be based on source revisions, not only image digests.

For a Git-backed stack:

- `Branch` means the tracked branch.
- `CommitSha == null` means track the branch head.
- `CommitSha != null` means pin to that commit and do not report branch-head updates.
- `ComposePaths` identify compose files relative to the repository root.
- `AdditionalEnvFileFromRepo` identifies extra env files relative to the repository root.

The deployed release must record the resolved commit that was applied. Do not use the same field both as "desired pinned commit" and "actual deployed commit"; that will make branch-tracking stacks accidentally become pinned after the first apply.

Release source metadata is mandatory before implementing Git-backed apply. Without it Citadel cannot answer:

- what commit is currently running
- whether the stack is outdated
- whether the release was pinned or branch-tracking
- which compose/env files were used
- what source revision to rollback to

Recommended shape:

```csharp
public sealed class StackReleaseSource
{
    public Guid? GitRepositoryId { get; init; }
    public string? GitRepositoryName { get; init; }
    public string? Branch { get; init; }
    public string? RequestedCommitSha { get; init; }
    public string ResolvedCommitSha { get; init; } = default!;
    public IReadOnlyList<string> ComposePaths { get; init; } = [];
    public IReadOnlyList<string> EnvFilePaths { get; init; } = [];
}
```

This can be stored as one JSON column on `StackReleases`, or as flat columns:

- `SourceType`
- `SourceRepositoryId`
- `SourceBranch`
- `SourceRequestedRevision`
- `SourceResolvedRevision`
- `SourceComposePathsJson`

The JSON object is simpler and keeps the table flexible. Flat columns are easier to query. Either way, `GitStack.CommitSha` remains user intent and `StackReleaseSource.ResolvedCommitSha` records runtime truth.

For manual stacks, source metadata can be null or use a lightweight manual source value later. Do not block Git support on modeling manual source history.

## Desired Apply Flow

Git stack apply should follow the same high-level lifecycle as manual stack apply, but source materialization must happen before Docker Compose execution.

1. lock stack control state
2. load the stack and current release
3. load the linked `GitRepository` with account
4. ensure the repository cache exists and is synced for the required branch
5. resolve the target commit:
   - explicit `CommitSha`, if present
   - otherwise current head of `Branch`
6. create an immutable snapshot directory for that exact commit
7. validate compose/env paths against the snapshot root
8. read and combine `ComposePaths`
9. read `AdditionalEnvFileFromRepo`
10. inject Citadel labels into the final compose content
11. build `StackApplyCommand`
12. execute the existing connector apply path
13. upsert runtime containers
14. save `StackReleaseSource`
15. release processing with `Healthy` or `Failed`

The key rule is: resolve the commit before reading files. Do not read compose files from a mutable current branch working tree.

The current `StackApplyCommand` already accepts compose content, env file path, env variables, registry auth, service names, pull behavior, and stack hooks. Git support should prepare those inputs before calling the same connector path, not fork Docker Compose execution.

Recommended materializer boundary:

```csharp
public interface IGitStackMaterializer
{
    Task<GitStackMaterializationResult> MaterializeAsync(
        Stack stack,
        GitStack spec,
        GitRepository repository,
        CancellationToken cancellationToken);
}

public sealed record GitStackMaterializationResult(
    string ResolvedCommitSha,
    string SourceBranch,
    string WorkingDirectory,
    string ComposeContent,
    string? EnvFilePath,
    IReadOnlyDictionary<string, string> EnvVars,
    IReadOnlyList<string> ComposePaths,
    IReadOnlyList<string> EnvFilePaths);
```

`ApplyStackService` should remain responsible for stack lifecycle, registry auth, connector execution, container upsert, status, notifications, and activity. The materializer should be responsible only for Git source resolution and file materialization.

## Git Source Path Rules

Path validation applies to:

- `ComposePaths`
- `AdditionalEnvFileFromRepo`
- repository-backed `EnvFilePath`, if supported for Git stacks

Reject:

- empty compose path list
- `../secret`
- `/etc/passwd`
- Windows absolute paths such as `C:\secret`
- non-existent files
- directories
- symlinks escaping the snapshot root
- paths that normalize outside the snapshot root

All accepted paths must be relative to the materialized repository snapshot root. Validation should resolve the full physical path and verify it remains under the snapshot root after symlink resolution.

`ComposePaths` should be required for Git stacks unless Citadel intentionally implements a deterministic auto-discovery rule. If auto-discovery is added later, it must fail when multiple candidate compose files exist.

## Branch And Commit Rules

`GitRepository.DefaultBranch` is the repository's default UI branch. `GitStack.Branch` is the branch tracked by a specific stack. They may differ.

Because stack branches may differ from the repository default, repository sync cannot only pull `DefaultBranch`. The Git cache layer must support branch-specific fetch/sync:

- sync default branch for repository health and preview
- sync stack branch before Git stack apply
- sync stack branch before checking Git stack update availability

Pinned commit apply must fail cleanly if the commit cannot be resolved locally or fetched from the remote.

Example failure:

```text
Git stack apply failed: commit abc123 was not found in repository citadel.
```

Do not silently fall back from a requested pinned commit to branch head.

## Concurrency

GitOps needs two independent locks:

- repository sync lock per repository
- stack apply lock per stack

The existing `RepoCacheManager` already has an in-memory per-repository semaphore. Keep that boundary, but make sure all cache mutations, including delete/reclone after source changes, go through the same lock.

Stack apply should continue using stack processing state. Auto-apply must not start while a manual apply, stop, pause, delete, or reconcile operation is running.

Repository patch/delete must not mutate or delete the cache while a sync is using it. The repository processing state and the repository cache lock should cover source changes and cache deletion.

## Desired Repository Sync To Stack Update Flow

When a repository sync succeeds:

1. record the latest resolved branch commit on the repository or sync result
2. find linked Git stacks where:
   - `Spec.GitRepoId == repository.Id`
   - `CommitSha == null`
   - `Branch` matches the synced branch
   - stack is not processing
   - current release is deployable or already deployed
3. compare latest repository commit to `StackReleaseSource.ResolvedCommitSha`
4. if different, update stack Git update state
5. emit an alert if the stack behavior is `Notify`
6. auto-apply the stack if behavior is `StackAutoDeploy`

Service-scoped auto deploy is not a good default for Git source changes because a commit can change networks, volumes, env files, compose structure, and service dependencies. For Git stacks, `ServiceAutoDeploy` should either be disabled in the UI or treated as `StackAutoDeploy` until Citadel can safely compute service-level compose diffs.

Current implementation:

- successful repository sync records a branch ref in `gitrepositoryrefs`
- branch-tracking Git stacks are loaded for the synced repository and branch
- pinned Git stacks are ignored
- stacks without a deployed Git release source are ignored until first apply
- `GitStackUpdateState.RecreateStackOnNewCommitState` records current and remote commits
- `Notify` creates `StackGitUpdateAvailable` activity and `StackGitUpdateAvailable` alert
- `StackAutoDeploy` auto-applies the full stack and emits Git-specific success/failure activity and alerts
- `ServiceAutoDeploy` is treated as full stack auto-deploy for Git source updates

## Update State

Manual stack auto-update uses image digest state. Git stacks need separate source update state.

Recommended state shape:

- current deployed commit
- latest repository commit
- branch
- checked time
- update available
- changed paths, if available later
- last sync/apply error, if any

This should be a `GitStackUpdateState` under the existing stack update-state model, separate from manual image update state.

For branch-tracking Git stacks, update state compares the branch head against `StackReleaseSource.ResolvedCommitSha`.

For pinned Git stacks, update state should usually be `UpToDate` or disabled because the user explicitly requested a fixed commit.

## Rollback Semantics

Git stack rollback should roll back to the previous `StackReleaseSource.ResolvedCommitSha`.

This is another reason release source metadata is not optional. Rollback should not mean "reapply whatever the branch points at now"; it should mean "reapply the exact source revision from a previous release."

Rollback can be implemented by creating a new release whose source request is the previous resolved commit. That preserves history and avoids mutating old release rows.

## Alerts

Git repository sync activities already exist, but there are no alert events for repository sync failures or Git stack update availability.

Recommended alert events:

- `GitRepositorySyncFailed`: repository clone/pull/auth failed.
- `StackGitUpdateAvailable`: linked repository has a newer commit than the deployed release.
- `StackGitAutoUpdated`: Git stack auto-apply succeeded.
- `StackGitAutoDeployFailed`: Git stack auto-apply failed.

Keep these separate from image update alerts. A new image digest and a new Git commit are different signals and should not share the same payload shape.

## Security And Operational Notes

Repository hooks are powerful. `OnClone` and `OnPull` execute shell commands from user configuration on the Citadel host. Treat them as privileged automation:

- require write/admin permission to configure them
- consider making them disabled by default
- add command timeout
- capture command output in activity details
- avoid exposing secrets in hook output
- keep path traversal protection

Webhook secrets should not be returned unmasked from normal list/detail views. Use config endpoints for editing secrets and return masked values elsewhere.

Git credentials should remain in `GitAccount`; stacks should only reference repositories. Avoid copying tokens or SSH keys into stack specs or releases.

## Current Issues To Fix Before GitOps Apply

- `GitReposRepository.AddAsync` receives control-state parameters but does not include control-state columns in the `INSERT` column list. `CreateGitRepository` calls `MarkProcessing`, but the processing state may not be persisted on create depending on database defaults.
- `PatchGitRepository` tries to delete the old cache path after source changes, but it compares `gitRepository.GetCachePath()` to itself after mutation. Capture the old cache path before `UpdateSource`.
- `RepoCacheManager.GetCachePath` can collide for repositories with the same repo name from different owners or hosts. Include repository id or a stable host/owner/repo hash in the cache path.
- `GitCliRepository.TestConnectionAsync` receives `repo.Url` directly. Relative repository names that rely on `GitAccount.Domain` should use the same resolved remote URL as clone/pull.
- `IGitCliRepository` comments describe bare clone, shallow fetch, internal refs, and materialized snapshots, but the current implementation uses a normal working tree clone and pull. Implement branch-specific fetch and commit materialization before Git stack apply, or reduce the comments to the current behavior until that work is done.
- `GitAccount.ValidateAuthType` has a duplicated token-auth match arm. It is harmless but should be cleaned up.
- `GitRepositoryView` exposes `WebHookSecret`. Normal views should mask it.
- `GitRepository` now owns repository sync mode and webhook secret. `GitStack` still has webhook fields from the earlier model; decide whether to remove or repurpose those before implementing webhook ingestion. Prefer repository-level webhook configuration, with stack-level opt-in policy if needed.

## Implementation Plan

Recommended order:

1. Stabilize repository sync persistence and cache identity.
2. Add explicit repository sync action in the UI and backend.
3. Add Git stack materialization service that resolves repository, branch/commit, compose paths, and env files into an apply-ready compose payload.
4. Extend `ApplyStackService` to support `GitStack` by using the materialization service and the existing connector apply path.
5. Add mandatory release source metadata so deployed Git revisions are reproducible.
6. Add branch-specific fetch and commit materialization to the Git cache layer.
7. Add `GitStackUpdateState`.
8. Add a background job that reacts to successful repository syncs and marks linked Git stacks with source updates.
9. Add Git stack update alerts.
10. Add optional auto-apply for branch-tracking Git stacks.
11. Add webhook ingestion later as another way to enqueue repository sync for repositories whose `SyncMode` is `Webhook`.

## Tests

Coverage should include:

- Git account auth/transport compatibility.
- repository URL/account domain validation.
- repository create persists processing state and queues sync.
- repository sync success marks healthy and records commit.
- repository sync failure marks degraded and records activity.
- repository cache path does not collide for same repo name from different owners.
- patching repository source deletes old cache and queues a new sync.
- Git stack create/patch rejects missing repositories.
- Git stack apply materializes compose files from repo cache.
- pinned Git stack does not report branch-head updates.
- branch-tracking Git stack reports update when repository head differs from deployed source revision.
- Git stack auto-apply records success/failure activities and alerts.

## Non-Goals For First Pass

- provider-specific webhook validation
- service-level deploy from arbitrary Git source diffs
- multi-repository stacks
- Git submodule support
- automatic PR creation or commit write-back
- secret management inside repository files
