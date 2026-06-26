# Git Stack Integration

## Purpose

Git stacks are Citadel stacks whose desired Docker Compose configuration is read from a Git repository, materialized at an immutable commit, and applied through the normal stack lifecycle.

This is the primary GitOps path for Citadel. It must support both common repository shapes:

- a repository dedicated to one stack with one compose file
- a monorepo containing many stack folders and many compose files

This spec focuses on the product and engineering contract. The lower-level repository sync and webhook listener details remain in `git-repository-gitops.md` and `webhook-listener.md`.

## References And Product Signals

The linked user reports show a consistent expectation:

- Homelab users often keep all compose stacks in one repository.
- Creating one Git-backed stack per compose folder should not clone the same repository once per stack.
- Selecting multiple compose files must not accidentally deploy unrelated compose files as one large stack.
- Users need clear examples for monorepo GitOps rather than a hidden procedure-only workaround.

References:

- https://opengitops.dev/
- https://docs.docker.com/compose/how-tos/multiple-compose-files/merge/

## Design Principles

- Git is the source of desired state for Git stacks.
- Releases are immutable. A stack release must record the exact commit and files that were applied.
- Citadel pulls and materializes Git content before applying it. Webhooks only notify Citadel that it should fetch and evaluate changes.
- One `GitRepository` cache is shared by many Git stacks. Do not clone a monorepo once per stack.
- A Citadel stack maps to one Docker Compose project. Multiple isolated compose apps in one monorepo are represented as multiple Citadel stacks linked to the same repository.
- A list of compose files on one Git stack means "base file plus overrides for this one Compose project", not "deploy each file as its own stack".
- Path handling must preserve Docker Compose relative-path behavior. Compose files, env files, build contexts, and relative bind mounts must resolve as users expect from their repository layout.
- Git stacks must not be implemented by loading compose YAML into `ComposeFileContent` or by concatenating compose files.
- Update detection must be path-aware so unrelated commits in a monorepo do not mark every linked stack outdated.

## Target User Model

Single-stack repository:

```text
repo/
  compose.yml
  .env
```

Citadel configuration:

- Git repository: `repo`
- Git stack: compose path `compose.yml`
- working directory: repository root

Monorepo:

```text
homelab/
  stacks/
    nas/
      beszel/
        compose.yml
        .env
        config/
      caddy/
        compose.yml
    media/
      jellyfin/
        compose.yml
  shared/
    networks.yml
```

Citadel configuration:

- one Git repository: `homelab`
- one Git stack for `stacks/nas/beszel/compose.yml`
- one Git stack for `stacks/nas/caddy/compose.yml`
- one Git stack for `stacks/media/jellyfin/compose.yml`

Each Git stack has its own Compose project name, platform, update policy, webhook config, drift policy, release history, activities, and alerts.

## Domain Model

### GitRepository

`GitRepository` owns repository connectivity and local source cache:

- URL
- default branch
- credentials through `GitAccount`
- sync mode and interval
- webhook config
- clone/pull hooks
- branch refs and latest resolved commit

The repository cache path should be unique by repository id or by a stable host/owner/repo hash. It must not be derived only from the repository name because two repositories named `homelab` can exist on different hosts or owners.

Recommended repository cache layout:

```text
/app/data/repos/{repositoryId}/cache
/app/data/repos/{repositoryId}/refs
```

The cache layer should support fetching any branch tracked by a linked stack, not only `GitRepository.DefaultBranch`.

Do not put the shared repository cache under one stack directory. A monorepo can feed many stacks, and repository cache cleanup should follow repository lifecycle, not stack release lifecycle.

### GitStack

`GitStack` owns how one stack consumes one repository. The model should make invalid states hard to represent:

- `GitRepoId`
- required `Branch`
- optional pinned `CommitSha`
- required ordered `ComposePaths`
- optional `WorkingDirectory`
- optional repository env file paths for Compose interpolation
- optional inline Compose interpolation values
- source update behavior
- project name
- registry
- hooks
- webhook config
- destroy-before-deploy behavior

Target shape, omitting the final `StackSpec` base-constructor wiring:

```csharp
public sealed record GitStack : StackSpec
{
    public required Guid GitRepoId { get; init; }
    public required string Branch { get; init; }
    public string? CommitSha { get; init; }
    public StackUpdateBehavior UpdateBehavior { get; init; }
    public string? ProjectName { get; init; }
    public StackWebhookConfig? Webhook { get; init; }
    public required IReadOnlyList<string> ComposePaths { get; init; }
    public string? WorkingDirectory { get; init; }
    public IReadOnlyList<string>? ComposeEnvFilesFromRepo { get; init; }
    public IReadOnlyDictionary<string, string>? ComposeEnvironment { get; init; }
    public IReadOnlyList<string>? WatchPaths { get; init; }
    public StackCommand? PreDeploy { get; init; }
    public StackCommand? PostDeploy { get; init; }
    public Guid? RegistryId { get; init; }
    public bool DestroyBeforeDeploy { get; init; } = true;
}
```

`WorkingDirectory` should default to the parent directory of the first compose path. This is the directory users expect when they write relative bind mounts, build contexts, local env files, and hook paths in a stack folder.

`WatchPaths` is optional. If empty, Citadel computes a default watch set from the working directory plus compose/env paths. Advanced users can narrow it for large monorepos.

Existing `EnvVars`, `EnvFilePath`, and `AdditionalEnvFileFromRepo` fields should be migrated carefully. For Git stacks, the target names are explicit:

- `ComposeEnvFilesFromRepo`: repository files passed to Docker Compose for interpolation.
- `ComposeEnvironment`: inline interpolation values passed to Docker Compose and the generated Citadel env file.

Container runtime `env_file:` remains owned by the Compose YAML. Citadel should not reinterpret repository env files as direct container runtime environment.

### StackReleaseSource

Release source metadata is mandatory for Git stacks.

Existing shape:

```csharp
public sealed record StackReleaseSource(
    StackSource SourceType,
    Guid? GitRepositoryId,
    string? GitRepositoryName,
    string? Branch,
    string? RequestedCommitSha,
    string ResolvedCommitSha,
    IReadOnlyList<string> ComposePaths,
    IReadOnlyList<string> EnvFilePaths);
```

Recommended additions:

```csharp
string? GitRepositoryUrl;
string? WorkingDirectory;
IReadOnlyList<string> WatchPaths;
IReadOnlyList<string> ComposeEnvFilesFromRepo;
string? ComposeDigest;
```

`RequestedCommitSha` records user intent. `ResolvedCommitSha` records runtime truth. For branch-tracking stacks, `RequestedCommitSha` is null and `ResolvedCommitSha` is the branch head at apply time. For pinned stacks, both can be set.

Do not store raw secrets in `StackReleaseSource`. Store only sanitized repository URLs, source paths, commits, and digests.

`ComposeDigest` must be defined before implementation. Recommended definition:

```text
SHA-256 over the final normalized Compose model for the resolved commit, after ordered Compose file merge, interpolation input selection, and Citadel label override injection.
```

Do not log or expose secret env values when computing or explaining the digest.

## Compose Path Semantics

`ComposePaths` is ordered. The first path is the base compose file. Later files are overrides for the same Compose project.

Valid examples:

```text
stacks/nas/beszel/compose.yml
stacks/nas/beszel/compose.yml + stacks/nas/beszel/compose.prod.yml
compose.yml + compose.override.yml
```

Invalid or discouraged examples:

```text
stacks/nas/beszel/compose.yml + stacks/nas/caddy/compose.yml
```

That example should be two Citadel stacks, not one stack with two compose paths.

Docker Compose evaluates relative paths for merged files relative to the base compose file. Citadel must not break that by concatenating YAML text into one file in an unrelated directory.

Required execution contract:

```bash
docker compose \
  --project-directory "{snapshotRoot}/{workingDirectory}" \
  -p "{projectName}" \
  --env-file "{generatedCitadelEnvFile}" \
  -f "{snapshotRoot}/{composePaths[0]}" \
  -f "{snapshotRoot}/{composePaths[1]}" \
  -f "{generatedCitadelLabelsOverrideFile}" \
  up -d
```

Rules:

- `ComposePaths` are ordered: first file is base, later files are overrides for the same Compose project.
- `WorkingDirectory` is the Compose project directory and defaults to the parent directory of the first compose path.
- The generated Citadel env file contains Citadel-managed Compose interpolation values only.
- Repository env files selected by Citadel are passed explicitly for Compose interpolation.
- The generated labels override file is always the final `-f` argument.
- The labels override file contains service labels only.
- The labels override file must not contain path-sensitive fields such as `build`, `volumes`, `env_file`, `extends`, or `include`.

For Git stacks, `StackApplyCommand` should carry:

- source working directory
- ordered compose file paths
- generated labels override file path
- generated env file path
- Compose environment values
- registry auth
- selected services, if supported

The current content-only `StackApplyCommand.ComposeFileContent` is sufficient for manual stacks, but it is too lossy for Git stacks that rely on repository-relative files.

## Environment Handling

Git stack environment must separate Compose interpolation from container runtime environment.

Rules:

- `ComposeEnvFilesFromRepo` are repository files passed to Docker Compose for interpolation.
- `ComposeEnvironment` values are passed to Docker Compose and may be written to a generated Citadel env file.
- Container runtime `env_file:` remains part of the Compose YAML and is resolved by Docker Compose relative to the project/compose context.
- Inline env values must be masked in logs and release views when they may contain secrets.
- Raw secrets must not be stored in `StackReleaseSource`.

## Path Validation Rules

All repository paths are relative to the materialized snapshot root.

Validate:

- `ComposePaths`
- `WorkingDirectory`
- `ComposeEnvFilesFromRepo`
- repository-backed `EnvFilePath`, if supported later
- `WatchPaths`
- hook command paths

Reject:

- empty compose path list
- empty or unresolved branch
- absolute paths
- Windows rooted paths such as `C:\secret`
- `..` traversal outside the snapshot root
- paths that normalize outside the snapshot root
- symlinks escaping the snapshot root
- missing files for compose/env paths
- directories where a file is required
- files where a directory is required

Do not rely on string prefix checks before resolving the final physical path. Resolve full paths and symlink targets, then verify they remain under the snapshot root.

## Materialization And Apply Flow

Git stack apply:

1. Lock stack processing state.
2. Load the stack, current release, platform, registry, and linked repository.
3. Acquire the repository sync lock.
4. Fetch the tracked branch, or fetch enough refs to resolve a pinned commit.
5. Resolve the target commit before reading any files.
6. Create a durable immutable source snapshot for the stack release.
7. Release the repository sync lock.
8. Validate `WorkingDirectory`, compose paths, env paths, watch paths, and hook paths against the snapshot.
9. Generate the Citadel env file, if needed.
10. Generate a labels-only Compose override file.
11. Validate the compose project by running Docker Compose config from the materialized source.
12. Apply through the existing stack connector lifecycle while holding only the stack processing lock.
13. Upsert runtime containers and images.
14. Persist `StackReleaseSource`.
15. Update Git stack update state.
16. Update `/app/data/stacks/{stackId}/current` atomically only after successful apply.
17. Record activity and alerts.
18. Release stack processing state.

Do not hold the repository sync lock while running Docker Compose. A slow deploy for one stack must not block every other stack linked to the same monorepo.

The snapshot must live under durable Citadel data, not the OS temp directory, because relative bind mounts may point to files in the materialized source directory for as long as containers are running.

Citadel already uses `/app/data/stacks` as the Docker Compose working root for manual stacks. Git stack source snapshots should live under that same stack storage root, but under a stable stack id path rather than the current stack name path. The existing manual stack connector writes generated files to `/app/data/stacks/{sanitizedStackName}`; that path is convenient for manual stacks but is not ideal for Git release snapshots because stack names can change and Git snapshots need release-level retention.

Recommended layout:

```text
/app/data/stacks/{stackId}/releases/{releaseId}/source
/app/data/stacks/{stackId}/current -> releases/{releaseId}/source
```

The connector should eventually accept a source working directory and ordered compose paths so Git stacks can run Compose from `/app/data/stacks/{stackId}/releases/{releaseId}/source` instead of rewriting repository content into `/app/data/stacks/{sanitizedStackName}/compose.yml`.

The release source snapshot must be a clean materialization of the resolved commit. Do not use the shared Git repository cache directly as the running stack source. Do not copy Git credentials into snapshots. Prefer exporting the resolved commit into a clean directory while preserving file contents, executable bits, and symlinks. Validate symlinks after export.

If apply fails, keep `/app/data/stacks/{stackId}/current` pointing at the previous successful release. Cleanup must not delete the materialized source used by currently running containers. Older release snapshots can be cleaned only after they are no longer active and rollback retention rules allow it.

## Update Detection

Repository sync updates branch refs. After a branch ref changes, Citadel evaluates linked Git stacks:

1. Find stacks with `GitRepoId == repository.Id`.
2. Ignore pinned stacks where `CommitSha` is set.
3. Match the stack branch.
4. Compare latest branch commit to `StackReleaseSource.ResolvedCommitSha`.
5. If the commit differs, compute changed paths between deployed commit and latest commit.
6. Intersect changed paths with the stack watch set.
7. If no relevant path changed, keep the stack up to date and record at most a low-noise activity.
8. If relevant paths changed, mark `GitStackUpdateState` as update available.
9. Emit `StackGitUpdateAvailable` activity and alert for notify mode.
10. Auto-apply the full stack for stack auto-deploy mode.

Default watch set:

- `WorkingDirectory/**`
- each `ComposePath`
- each `ComposeEnvFilesFromRepo`
- any explicit `WatchPaths`

This default favors developer experience for monorepos because config files, build contexts, and local assets stored beside the compose file are treated as relevant. Large repositories can narrow the watch set explicitly.

If the Compose project references shared files outside `WorkingDirectory`, Citadel should detect and add them to the effective watch set when possible. At minimum, the preview endpoint should warn:

```text
This stack references files outside its working directory. Add them to WatchPaths to receive updates when they change.
```

Changed-path computation must fail safe. If diff calculation is unreliable because of force-pushes, missing old commits, deleted files, renames, or first deployment, treat the stack as potentially affected instead of silently up to date.

## Webhook Integration

Webhooks must use the same sync and update path as polling.

Repository webhook:

1. Authenticate provider payload if a secret is configured.
2. Validate branch and repository identity.
3. Fetch the repository branch.
4. Update branch refs.
5. Evaluate all linked Git stacks with path-aware update detection.
6. Auto-apply only stacks whose policy permits it.

Stack webhook:

1. Authenticate provider payload if a secret is configured.
2. Require a Git stack.
3. Reject pinned Git stacks unless `ForceDeploy` is explicitly intended to reapply the pinned commit.
4. Validate branch and repository identity.
5. Fetch the linked repository branch.
6. Resolve the target commit from the fetched branch.
7. If `ForceDeploy` is false, deploy only when relevant watched paths changed.
8. Apply the stack from the resolved commit.

Never deploy directly from the commit SHA supplied by the webhook payload without fetching and verifying it in the linked repository cache.

## UI Design

The stack form should make Git stacks feel like selecting source, not typing infrastructure internals.

Git stack fields:

- repository selector
- branch selector populated from synced refs
- optional commit pin
- compose file picker
- ordered override file picker
- working directory preview, editable in advanced mode
- env file picker
- watch paths in advanced mode
- update behavior
- webhook config under advanced, shown only for Git stacks

Monorepo compose picker:

- shows a repository tree or searchable path picker
- highlights likely compose files such as `compose.yml`, `compose.yaml`, `docker-compose.yml`, and `docker-compose.yaml`
- lets the user create one stack from one compose file
- lets the user add override files to the same stack
- warns when selected compose files live in unrelated directories
- previews detected services before save/apply

Optional later wizard:

- "Discover stacks from repository"
- scans for compose candidates
- lets the user select many candidates
- creates one Citadel stack per selected candidate
- assigns platform, branch, update behavior, and naming pattern in bulk

The stack table should show:

- source repository
- branch
- current short commit
- remote short commit when update is available
- update state
- processing state

The stack detail page should include a `Releases` tab:

- list historical releases with version, status, source repository, branch, resolved commit, compose paths, and created time
- mark the current release clearly
- allow rollback only from previous healthy releases
- hide failed apply attempts from release history; failed attempts belong in activities/logs, not rollbackable release history
- stream rollback progress through the same task sheet used by stack apply
- explain that rollback creates a new release from historical source instead of mutating old releases

## Activities And Alerts

Activities:

- `StackGitUpdateAvailable`
- `StackGitAutoUpdated`
- `StackGitAutoDeployFailed`
- `StackApplied` with source metadata
- `StackWebhookReceived`
- low-noise `StackGitNoRelevantChanges` can be added later if users need audit detail

Alerts:

- `StackGitUpdateAvailable`
- `StackGitAutoDeployFailed`
- webhook authentication and dispatch alerts from `webhook-listener.md`

Do not alert for every repository sync, branch mismatch, webhook no-op, or unrelated monorepo commit.

## Rollback

Rollback must use the previous release's full `StackReleaseSource`, not the current branch head and not the current `GitStack` settings.

Use from the previous release:

- `ResolvedCommitSha`
- `ComposePaths`
- `ComposeEnvFilesFromRepo`
- `WorkingDirectory`
- `WatchPaths`
- `ComposeDigest`, if recorded

Rollback should create a new release from that previous source revision. It should not mutate old release rows. If the commit is missing from the local cache, Citadel should fetch it from the remote and fail clearly if it cannot be resolved.

This matters because current stack config may point to paths that did not exist in the old commit.

Failed apply attempts must not advance the release version on every retry. A failed current release is retried in place until it succeeds or the user changes configuration. The rollback list returns previous healthy releases only; the current release and runtime-degraded releases are not rollback targets.

## Security

- Keep Git credentials in `GitAccount`; never copy secrets into stack specs or releases.
- Treat repository hook commands as privileged host automation.
- Enforce command timeouts and path validation for hook working directories.
- Mask webhook secrets in normal views.
- Do not trust webhook branch, commit, or repository fields without validating against the fetched repository.
- Do not execute files from the repository automatically. Only user-configured hooks run.
- Preserve AOT-friendly JSON source generation for all new config and source metadata types.

## Hook Command Contract

The existing `StackCommand` shell-command list is convenient but too implicit for Git-backed automation. The target shape should be explicit and safe by default:

```csharp
public sealed record StackCommand(
    string Executable,
    IReadOnlyList<string> Arguments,
    string? WorkingDirectory,
    TimeSpan Timeout,
    IReadOnlyDictionary<string, string>? Environment,
    bool UseShell = false);
```

Rules:

- Default execution does not use a shell.
- Shell execution, if supported, must be explicit and shown as privileged in the UI.
- Hook working directories must pass snapshot-root validation for Git stacks.
- Hook stdout/stderr must be size-limited and secret-masked.

## First Slice Boundary

Implement first:

1. shared `GitRepository` cache
2. branch and pinned commit resolution
3. durable release snapshot under `/app/data/stacks/{stackId}/releases/{releaseId}/source`
4. path validation against the snapshot root
5. Compose execution with ordered `-f` paths
6. generated labels override file as the final `-f` argument
7. `StackReleaseSource` persistence
8. path-aware update detection
9. rollback from previous release source

Defer:

- UI monorepo discovery wizard
- submodules
- multi-repository stacks
- secret manager integration
- durable webhook delivery history
- generated pull requests or write-back to Git
- service-level deploy from arbitrary Git diffs

## Test Coverage

Unit tests:

- path validation rejects traversal, rooted paths, symlink escapes, missing files, and directory/file mismatches
- working directory defaults to the parent of the first compose path
- compose path selection rejects empty lists
- branch validation rejects empty or unresolved branches
- update detection ignores pinned stacks
- update detection ignores unrelated monorepo path changes
- update detection marks relevant working-directory changes
- update detection treats force-pushes, missing old commits, deleted files, renames, and first deployment as potentially affected
- release source metadata records requested and resolved commits separately
- release source metadata stores sanitized repository URL and never stores raw secrets
- hook command shell behavior is opt-in

Integration tests:

- one Git repository can back multiple Git stacks without multiple clones
- applying a single-compose repository persists source metadata
- applying a monorepo stack uses files relative to its stack folder
- applying ordered compose overrides preserves Docker Compose merge behavior
- Docker Compose receives ordered `-f` files without YAML concatenation
- generated labels override file is the final `-f` argument and contains labels only
- default `.env` lookup comes from `WorkingDirectory`
- `--project-directory` is set to the materialized working directory
- failed deploy does not update `/app/data/stacks/{stackId}/current`
- repository sync lock is released before Compose apply
- repository polling marks only affected linked stacks outdated
- repository webhook fetches first, then evaluates linked stacks
- stack webhook deploys from fetched resolved commit
- rollback applies the previous release source paths, not current stack paths

Frontend tests:

- Git stack form shows Git-only fields only for Git stacks
- compose path picker updates form dirty state
- webhook config participates in normal preview/save flow
- stack table displays current and remote commit state

## Non-Goals For First Pass

- multi-repository stacks
- generated pull requests or write-back to Git
- submodule support
- service-level deploy from arbitrary Git diffs
- secret manager integration for repository env files
- durable webhook delivery history
