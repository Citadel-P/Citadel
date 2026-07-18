# Citadel Builds

This is the implementation specification for first-class Docker/OCI image builds in Citadel.

Implement it one slice at a time.

If no slice is explicitly requested:

1. Implement **Slice 1 only**.
2. Compare this specification with the current codebase before changing files.
3. Reuse existing Citadel patterns and services instead of creating parallel abstractions.
4. Keep endpoints thin and put behavior in Mediator commands, queries, and application services.
5. Run relevant backend tests, migrations, OpenAPI generation, frontend API generation, and frontend build checks.
6. Report implemented behavior, schema changes, generated artifacts, tests, deferred work, and conflicts between this specification and the codebase.
7. Do not continue automatically into another slice.

---

# 1. Goal

Add a first-class **Builds** resource that lets users:

- select an existing Git repository and branch
- build a Dockerfile on an existing Citadel platform
- push the resulting OCI image to an existing Citadel registry
- follow live logs and inspect previous runs
- record the exact source commit, image references, and digest
- cancel or time out an active build safely

Builds are not Automation scripts.

Automation Actions may call the public Build API, but they must not receive direct access to Docker sockets, registry credentials, Git credentials, secret values, or arbitrary host paths.

---

# 2. Slice 1 Scope

Slice 1 implements only:

- build project CRUD
- Git repository source
- local, regular-agent, and edge-agent platform execution
- Dockerfile builds through a Citadel build helper
- literal or variable-backed build arguments
- BuildKit secret mounts from existing Citadel secrets
- registry push
- manual runs
- live and persisted logs
- cancellation and timeout
- run history
- activities
- permissions and capabilities
- resource tags
- basic frontend UI

Slice 1 does **not** implement:

- schedules
- webhooks
- deployment or stack updates
- dedicated build agents or builder pools
- Citadel-managed AWS/VM provisioning
- arbitrary shell steps
- visual pipelines
- Docker Compose build orchestration
- multi-platform builds
- Kubernetes runners
- image scanning
- SBOM generation
- provenance or signing
- registry promotion
- registry image cleanup
- Git repository write-back

Do not add placeholder tables, fields, endpoints, or UI for deferred features.

---

# 3. Product Surface

Menu:

```text
Builds
```

Frontend routes:

```text
/builds
/builds/add
/builds/edit/{id}
/builds/{id}/runs/{runId}
```

## Builds List

Show:

- name
- Git repository and branch
- target platform
- output repository
- current or last run status
- last run time
- tags

Support the existing search, tag filtering, pagination, permissions, and resource-list patterns.

## Build Project Page

Header:

- inline editable name and description
- enabled/disabled state
- current or last run status
- actions: **Build**, **Cancel**, **Enable/Disable**, **Delete**

Tabs:

- **Config**
- **Runs**
- **Activities**

## Run Page

Show:

- status
- trigger
- source repository, branch, and resolved commit
- platform snapshot
- registry and image repository
- pushed tags
- canonical image references
- digest when available
- queued, started, and finished timestamps
- duration
- exit code and error message
- live/persisted logs
- cancel action while active

---

# 4. Citadel Alignment

## Backend

Follow existing Citadel conventions:

- public API routes live under `/api/v1`
- register routes from `src/Citadel.WebApi/Routes/PublicEndpoints.cs`
- use Mediator commands, queries, and stream/event handlers
- use typed domain entities and source-generated JSON contexts
- use existing Dapper/EF migration conventions
- use existing actor, permission, capability, activity, tag, and SignalR patterns
- use existing Git repository materialization services
- use existing platform connector abstractions
- use existing registry credential resolution
- use existing secret resolution and redaction

A build project is configuration, not a reconciled runtime resource.

Do not add a second persisted project status or make `BuildProject` an `IReconcilableResource`. The active and last status comes from `BuildRun`.

## Frontend

Reuse:

- resource shell and page layout
- form builder and validation patterns
- Git repository, platform, registry, secret, and tag selectors
- capability-based action visibility
- run history tables
- activity tab
- live log viewer

Do not implement a script editor or generic pipeline editor.

---

# 5. Core Concepts

## Build Project

A `BuildProject` is reusable configuration describing:

- the Git source to build
- the Docker build configuration
- the platform that executes the build
- the registry repository and tags to push
- timeout and run retention

## Build Run

A `BuildRun` is an immutable execution attempt.

A run snapshots all non-secret information required to understand what happened, even if the project, repository, platform, or registry is later changed.

## Build Helper

A versioned Citadel build-helper image performs the actual BuildKit build and registry push on the selected platform.

The helper is an internal implementation detail. Users do not select its image, network mode, extra hosts, privileges, or command-line flags in Slice 1.

---

# 6. Domain Model

Slice 1 adds only:

- `BuildProject`
- `BuildRun`
- `BuildRunLog`

Suggested table names, following the existing lowercase/plural convention:

- `buildprojects`
- `buildruns`
- `buildrunlogs`

Use the existing generic resource-tag tables. Do not add build-specific tag tables.

## 6.1 BuildProject

Required fields:

```text
Id
Name
Description?
Enabled
GitRepositoryId
Branch
ContextPath
DockerfilePath
Target?
BuildArgs
BuildSecrets
PlatformId
RegistryId
ImageRepository
TagTemplates
TimeoutSeconds
RetentionRunCount
CurrentRunId?
CreatedByActorId
CreatedAt
UpdatedAt
RowVersion
```

Defaults:

```text
Enabled = true
Branch = linked repository default branch
ContextPath = "."
DockerfilePath = "Dockerfile"
TagTemplates = ["{branch}-{shortSha}"]
TimeoutSeconds = 1800
RetentionRunCount = 20
```

Validation:

- name follows existing resource-name rules
- the linked Git repository, platform, and registry must exist
- branch is required after default resolution
- `ContextPath` and `DockerfilePath` are relative to repository root
- absolute paths and `..` traversal are rejected
- the resolved Dockerfile must be inside the selected build context in Slice 1
- `ImageRepository` contains only the registry repository path, with no registry host, tag, or digest
- at least one tag template is required
- resolved tags must be valid OCI/Docker tags
- duplicate resolved tags are removed while preserving order
- timeout and retention use bounded server-side validation

`CommitSha` is not stored on the project.

A project follows a branch. A manual run may optionally request a specific commit; every run always stores the resolved commit.

### Build Args

```csharp
public sealed record BuildArgSpec(
    string Name,
    string? LiteralValue,
    Guid? ResourceBindingId);
```

Rules:

- exactly one of `LiteralValue` or `ResourceBindingId` is set
- binding-backed args may resolve only non-secret variables
- build arguments are not treated as secret because Docker build args can leak into image history or metadata
- sensitive values must use BuildKit secrets instead

### Build Secrets

```csharp
public sealed record BuildSecretSpec(
    string Id,
    Guid SecretId);
```

The Dockerfile consumes them with BuildKit syntax, for example:

```dockerfile
RUN --mount=type=secret,id=npmrc \
    cp /run/secrets/npmrc ~/.npmrc && npm ci
```

Rules:

- secret values are resolved only when the run starts
- values are materialized as temporary files inside the helper
- values are never stored in project snapshots, run snapshots, logs, activities, or image labels
- only the configured secret ids and BuildKit mount ids may be stored
- duplicate secret mount ids are rejected

## 6.2 BuildRun

Required fields:

```text
Id
BuildProjectId
BuildProjectName
Trigger
Status
TriggeredByActorId
QueuedAt
StartedAt?
FinishedAt?
DurationMs?
SourceSnapshot
PlatformSnapshot
OutputSnapshot?
ExitCode?
ErrorMessage?
LogsTruncated
TimeoutSeconds
```

Use typed JSON snapshots.

```csharp
public sealed record BuildSourceSnapshot(
    Guid GitRepositoryId,
    string RepositoryName,
    string RepositoryUrl,
    string Branch,
    string ResolvedCommitSha,
    string ContextPath,
    string DockerfilePath,
    string? Target,
    IReadOnlyList<string> BuildArgNames,
    IReadOnlyList<string> BuildSecretIds);

public sealed record BuildPlatformSnapshot(
    Guid PlatformId,
    string PlatformName,
    PlatformConnectorType ConnectorType);

public sealed record BuildOutputSnapshot(
    Guid RegistryId,
    string RegistryName,
    string RegistryServer,
    string ImageRepository,
    IReadOnlyList<string> Tags,
    IReadOnlyList<string> ImageReferences,
    string? Digest);
```

Snapshots must not contain credentials, secret values, resolved variable values, temporary paths, or authentication headers.

### Run Status

```csharp
public enum BuildRunStatus
{
    Queued,
    Preparing,
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    Interrupted
}
```

`Interrupted` is used when Citadel restarts or loses the active execution session and cannot prove that the run completed.

Do not create a persisted `Rejected` run for validation, permission, disabled-project, or concurrency failures. Return the appropriate API error without creating a run.

### Trigger

```csharp
public enum BuildRunTrigger
{
    Manual,
    Automation
}
```

The server determines the trigger. Clients cannot choose an arbitrary trigger value.

Future slices may append `Schedule` and `Webhook`.

## 6.3 BuildRunLog

Store bounded log chunks, not one database row per character or process byte.

Required fields:

```text
Id
BuildRunId
Sequence
Kind
Text
CreatedAt
```

```csharp
public enum BuildRunLogKind
{
    StdOut,
    StdErr,
    System
}
```

Rules:

- `Sequence` is strictly increasing per run
- add a unique index on `(BuildRunId, Sequence)`
- stream chunks to SignalR before or while persisting them
- batch database writes where the existing infrastructure permits it
- preserve ANSI output if the existing log viewer supports it
- redact configured secrets before streaming and persistence
- enforce a server-wide maximum persisted bytes per run
- when the limit is reached, persist one truncation message, set `LogsTruncated = true`, stop persisting more chunks, but continue the live stream

---

# 7. Source Resolution And Transfer

Slice 1 supports existing Citadel Git repositories only.

## Resolution

For every run:

1. resolve the project branch or optional requested commit
2. sync/fetch through the existing Git repository service
3. resolve one exact commit SHA
4. create the immutable source snapshot
5. materialize that exact commit as a tar archive
6. send the archive to the selected platform through the platform connector

Use `git archive` or the existing equivalent snapshot service so that:

- the build contains committed files from the resolved commit
- `.git` is not included
- uncommitted Core filesystem changes are not included
- the helper does not receive Git credentials

The helper must never clone the repository itself in Slice 1.

This gives local, regular-agent, and edge-agent builds the same source behavior and keeps Git credentials inside the existing Citadel Git integration.

## Context Rules

- archive only the selected context when the current Git materializer supports it safely; otherwise archive the commit and extract the validated context inside the helper
- honor `.dockerignore` through BuildKit
- reject missing context or Dockerfile paths before starting BuildKit
- submodules and Git LFS follow the current Git repository capability; do not add new submodule/LFS behavior solely for Builds
- enforce the existing transfer limit, or add one narrow `Builds:MaxContextBytes` setting if no suitable limit exists

Do not depend on host paths being shared between Citadel Core and the selected Docker platform.

---

# 8. Platform Build Execution

Slice 1 stores `PlatformId` directly on `BuildProject`.

Do not introduce a polymorphic builder hierarchy until a second builder kind is actually implemented.

## Helper Image

Use a server-configured, versioned Citadel build-helper image, following the same pull/version pattern as existing backup or volume helpers.

Example configuration key:

```text
Builds:HelperImage
```

Do not expose a per-project helper image override in Slice 1.

The helper contains:

- BuildKit
- source archive extraction tools
- the small Citadel helper entrypoint
- registry authentication support
- structured result output

BuildKit is mandatory in Slice 1. Do not implement a separate legacy `docker build` path.

## Execution Flow

1. Resolve and snapshot the source.
2. Resolve build args and secrets.
3. Resolve registry login material through the existing registry service.
4. Create the helper container on the selected platform through `IContainerConnector`.
5. Transfer the source archive and temporary secret/authentication files.
6. Start the BuildKit build and push all resolved tags.
7. Stream stdout, stderr, and system messages to Core.
8. Read a structured helper result containing exit code, image references, and digest.
9. Persist the output snapshot.
10. Remove the helper container and temporary material in `finally`.

The helper pushes directly to the registry. It does not need to import the image into the selected platform Docker daemon.

Do not add the pushed image to platform-local image inventory unless the image is actually present in that daemon.

## Helper Result

Do not parse the image digest from human-readable console text.

The helper must return structured result data, for example:

```json
{
  "exitCode": 0,
  "digest": "sha256:...",
  "references": [
    "ghcr.io/acme/api:main-a4c8e3c1"
  ]
}
```

The transport may use the existing connector result channel or a known result file copied back from the helper.

## Helper Identification And Cleanup

Label helper containers with at least:

```text
io.citadel.managed=true
io.citadel.resource=build-run
io.citadel.build-run-id={runId}
io.citadel.build-project-id={buildProjectId}
```

Cancellation, timeout, failure, and Core startup recovery use these labels to find and remove orphaned helpers.

---

# 9. Registry And Image Output

A build pushes to one existing Citadel registry and one image repository.

Project fields:

```text
RegistryId
ImageRepository
TagTemplates
```

The registry host comes from the selected registry resource and must not be duplicated in `ImageRepository`.

Example:

```text
Registry: ghcr.io
ImageRepository: acme/api
TagTemplates:
  - {branch}-{shortSha}
  - latest
```

Resolved output:

```text
ghcr.io/acme/api:main-a4c8e3c1
ghcr.io/acme/api:latest
```

Supported tag placeholders:

```text
{branch}
{shortSha}
{sha}
{runId}
{date:yyyyMMdd}
{project}
```

Rules:

- placeholder values are normalized to valid tag components
- invalid characters are replaced consistently
- a resolved tag must be non-empty and within the OCI/Docker tag limit
- duplicate tags are removed
- the first resolved reference is the primary reference shown in lists
- credentials come only from the selected registry resource
- registry credentials are never copied into project or run snapshots

Use the digest reported by BuildKit after the push. A digest may be absent only when the selected registry/build output cannot return one; the run may still succeed, but the UI must show that no digest was reported.

---

# 10. Run Queue, Concurrency, Cancellation, And Recovery

## Queueing

`POST /api/v1/buildProjects/{id}/runs`:

1. validates project state and the current actor
2. validates linked dependencies
3. atomically claims the project by setting `CurrentRunId`
4. inserts a queued run in the same transaction
5. returns the run

If the project is disabled or already has an active run, return `409 Conflict` and do not create another run.

## Concurrency

- allow one active run per build project
- different projects may run concurrently
- protect Core and agents with a server-wide `Builds:MaxConcurrentRuns` capacity setting
- capacity is an operational setting, not a license feature

Use an atomic database update or equivalent existing locking pattern. Do not rely only on an in-memory check.

## Worker

Use a durable database-backed queued state.

A hosted worker should pick queued runs and execute them. Queued runs must not disappear when Core restarts before execution.

## Cancellation

Cancellation is allowed while a run is:

- `Queued`
- `Preparing`
- `Running`

Cancellation:

- records the user request
- cancels the execution token
- stops and removes the helper container
- keeps existing logs and snapshots
- finishes the run as `Cancelled`
- clears `CurrentRunId` only when it still points to that run

## Timeout

- enforce the project timeout across preparation, transfer, build, and push
- on timeout, cancel and remove the helper
- finish as `TimedOut`

## Core Restart Recovery

On startup:

- queued runs remain queued and may resume normally
- runs left in `Preparing` or `Running` become `Interrupted`
- clear stale `CurrentRunId` references
- attempt best-effort cleanup of helper containers identified by Citadel labels
- record an activity describing the interruption

Do not silently mark an unknown interrupted build as succeeded.

---

# 11. Security

## Credentials

- Git credentials stay inside the existing Git repository integration
- registry credentials are resolved only for the active run
- secret values are resolved only for the active run
- pass registry authentication through a temporary Docker configuration or the existing safe equivalent
- never include secrets in command-line arguments when a file or environment channel is available
- remove all temporary credential and secret files with the helper container

If the current registry model cannot produce generic login material, add one narrow application service such as:

```csharp
public interface IRegistryLoginMaterialResolver
{
    ValueTask<RegistryLoginMaterial> ResolveAsync(
        Guid registryId,
        CancellationToken cancellationToken);
}
```

The returned type must be used only in memory and must have a redacted `ToString()` representation.

## Redaction

Reuse the existing secret redactor.

Redact at least:

- selected BuildKit secret values
- registry passwords and tokens
- temporary generated tokens
- any resolved value marked secret by the existing secret system

Apply redaction before:

- SignalR publication
- database persistence
- activity/error persistence
- structured application logging

BuildKit secrets reduce accidental exposure, but a malicious Dockerfile may still print a secret. Exact-value redaction remains required.

## Access Boundary

Automation Actions may queue a build and read its status through the public API.

They must not receive APIs for:

- starting arbitrary helper containers
- mounting Docker sockets
- resolving raw registry credentials
- resolving raw Git credentials
- reading raw secret values
- copying arbitrary host files

---

# 12. Permissions And Capabilities

Add `Build` as a first-class resource type.

Use the existing generic resource-action model rather than introducing parallel permission names such as `Build_View` or `Build_Run`.

Expected actions:

```text
View
Create
Update
Delete
Apply
```

For Builds, `Apply` means queue or cancel a run.

Use existing capability naming and mapping. The frontend needs at least:

```text
canRead
canWrite
canDelete
canApply
canCancel
canViewRuns
canViewActivities
```

Dependency access must follow the same rule already used by Deployments and Stacks. At minimum, the execution actor must still be allowed to use the linked Git repository, platform, registry, and selected secrets.

Do not invent build-specific dependency actions when an existing contextual permission rule already covers the same operation.

Validate dependency access:

- when creating or updating the project
- before queueing a manual run
- again in the worker before resolving credentials or starting the helper

If access was lost after queueing, fail the run before helper execution with a safe error message.

For Slice 1, the execution actor is the actor who queued the run. A future schedule/webhook slice may add a configured execution actor.

---

# 13. API Surface

Use the exact casing and response conventions of existing Citadel public routes.

Required Slice 1 routes:

```text
GET    /api/v1/buildProjects
POST   /api/v1/buildProjects
GET    /api/v1/buildProjects/{id}
PATCH  /api/v1/buildProjects/{id}
DELETE /api/v1/buildProjects/{id}

POST   /api/v1/buildProjects/{id}/runs
GET    /api/v1/buildProjects/{id}/runs

GET    /api/v1/buildRuns/{runId}
GET    /api/v1/buildRuns/{runId}/logs
POST   /api/v1/buildRuns/{runId}/cancel
```

Manual run request:

```csharp
public sealed record RunBuildProjectRequest(
    string? CommitSha = null);
```

Rules:

- an omitted commit builds the current head of the configured branch
- a supplied commit must be reachable from or valid for the linked repository according to the existing Git security rules
- clients cannot override platform, registry, image repository, tags, build args, or secrets per run in Slice 1

Log retrieval should support the existing pagination pattern or an `afterSequence` cursor so a reconnecting client can fetch missing chunks.

Use SignalR for live run/status/log updates. Do not add a second SSE or custom HTTP streaming endpoint in Slice 1.

Automation uses the same generated public API:

```ts
const run = await citadel.buildProjects.run(id, { commitSha })
const state = await citadel.buildRuns.get(run.id)
```

---

# 14. Activities And Events

Slice 1 activity types:

```text
BuildProjectCreated
BuildProjectUpdated
BuildProjectDeleted
BuildRunQueued
BuildRunStarted
BuildRunSucceeded
BuildRunFailed
BuildRunTimedOut
BuildRunCancelled
BuildRunInterrupted
```

Activities should include useful non-secret metadata:

- build project id/name
- run id
- trigger
- resolved commit
- primary output reference
- digest when available
- duration
- safe failure summary

Do not create one activity per log line.

Publish list/detail/run updates through the existing SignalR manager pattern.

Suggested logical groups, adapted to current naming conventions:

```text
builds
build:{buildProjectId}
build-run:{buildRunId}
```

---

# 15. Retention

Each project keeps its most recent `RetentionRunCount` terminal runs.

Defaults:

```text
20 runs
minimum 1
```

After a run becomes terminal:

1. determine runs older than the retained count
2. delete their persisted log chunks
3. delete their run rows according to existing resource deletion conventions

Retention does not delete:

- images from registries
- Git commits
- platform images
- activities required by the existing audit model

Project deletion must be rejected while a run is active. Otherwise follow the existing resource deletion and activity conventions.

---

# 16. Frontend Config Form

Use four small sections.

## Source

- Git repository
- branch
- context path
- Dockerfile path
- optional target stage

## Build Inputs

- build arguments: name + literal or variable binding
- BuildKit secrets: mount id + secret selector

Explain in the UI that build arguments are not secret and may be visible in image metadata.

## Output

- registry
- image repository
- tag templates list

Show a preview using placeholder sample values.

## Execution

- platform
- timeout
- retained runs
- enabled
- tags

Do not expose helper-image, network, privilege, extra-host, or raw BuildKit flags in Slice 1.

---

# 17. Error Handling

Return clear errors for:

- disabled project
- active run already exists
- repository, branch, or commit not found
- invalid or missing context/Dockerfile
- context exceeds configured transfer limit
- inaccessible platform
- helper image pull/start failure
- registry authentication failure
- invalid resolved tag
- BuildKit failure
- registry push failure
- timeout
- cancellation
- dependency access lost

Persist only a safe user-facing error message on the run.

Detailed exceptions may go to structured server logs after redaction, following current Citadel logging conventions.

Always clean up helper resources in `finally`.

---

# 18. Tests

Add tests consistent with the current test architecture.

## Domain/Validation

- path traversal and absolute path rejection
- Dockerfile-inside-context validation
- build arg source exclusivity
- secret mount id uniqueness
- tag placeholder resolution and normalization
- duplicate tag removal
- timeout and retention bounds

## API/Permissions

- CRUD permissions
- dependency access checks
- disabled project returns conflict
- concurrent run returns conflict
- delete active project returns conflict
- clients cannot choose trigger or override protected run configuration

## Execution

Use a fake connector/helper contract where practical.

Cover:

- successful source resolution, build, push, and digest persistence
- source snapshot uses the exact resolved commit
- local, regular-agent, and edge-agent routes use the same execution service
- failed build
- failed push
- cancellation
- timeout
- helper cleanup
- startup interruption recovery
- current run claim is cleared safely
- queued work survives worker restart semantics

## Security/Logs

- secret values never appear in project/run snapshots
- secret values are redacted from stdout, stderr, errors, and application logs
- persisted log cap marks truncation while live logs continue
- digest and references come from structured helper output, not log parsing

## Frontend

- create/edit form validation
- capability-based actions
- current/last status rendering
- run history
- live log reconnect using persisted sequence
- frontend API generation and production build

---

# 19. Slice 1 Acceptance Criteria

Slice 1 is complete when:

1. A permitted user can create a build project from an existing Git repository.
2. The user can select a branch, context, Dockerfile, platform, registry, repository path, and tag templates.
3. The user can configure non-secret build args and BuildKit secrets.
4. The user can manually queue one run when the project has no active run.
5. Citadel resolves and records the exact commit before execution.
6. Core sends the immutable source archive to a helper on the selected local, regular-agent, or edge-agent platform without relying on shared host paths.
7. The helper builds with BuildKit and pushes every resolved image tag.
8. The run records status, timestamps, logs, source snapshot, platform snapshot, output references, digest when available, and safe failure information.
9. Logs are visible live and after reconnect.
10. Cancellation and timeout stop and remove the helper.
11. Core restart marks unknown active runs interrupted and clears stale claims.
12. No Git, registry, or secret value is persisted or exposed to Automation Actions.
13. Activities, tags, permissions, API generation, migrations, backend tests, and frontend build checks pass.

---

# 20. Future Slices

Future slices are intentionally described only at product level. Specify them in detail when implementation begins.

## Slice 2: Webhooks, Scheduling, And Failure Alerts

Add:

- shared listener-based GitHub/GitLab webhooks
- repository, branch, and path filtering
- one optional cron/timezone schedule
- configured execution actor for non-interactive runs
- build failure/timeout alerts

## Slice 3: Deployment And Stack Consumers

Add:

- update a deployment image reference
- optionally apply the deployment
- update a stack resource binding such as `IMAGE_REF`
- optionally apply the stack
- never edit Git-backed stack files
- prefer immutable digest references for automatic production deployment

## Slice 4: Dedicated Build Agents

Add a dedicated build-agent resource that reuses the outbound Edge Agent transport and executes builds without being a general Docker platform.

Cloud VM provisioning is outside the initial build-system scope. Users or infrastructure automation may launch build agents on AWS, Azure, GCP, or on-premises infrastructure.

Do not add an AWS-specific EC2 provisioning model unless there is a separate validated product requirement for Citadel to own cloud instance lifecycle.

## Slice 5: Advanced Outputs

Potential additions:

- multi-platform images
- SBOM
- provenance
- signing
- vulnerability scanning
- explicit image retention and promotion policies

---

# 21. Licensing

Do not gate Slice 1.

Potential paid value later:

- dedicated build agents
- scheduled builds
- advanced supply-chain outputs
- managed cloud-builder integrations, if ever implemented

Do not license concurrency as a product feature. Control it through operational capacity settings.

---

# 22. Final Decisions

The following questions are resolved for this specification:

- Resource type: use **Build**, not **Builder**. A builder is an execution target, while a build is the user resource.
- Slice 1 builder model: store `PlatformId` directly; do not add a polymorphic builder abstraction yet.
- Build engine: BuildKit is mandatory.
- Secret build args: not supported; use BuildKit secrets.
- Tags: use one ordered `TagTemplates` list; users add `latest` explicitly instead of a separate `PushLatest` flag.
- Source checkout: Core resolves/materializes the exact Git commit and transfers an archive; the helper never clones Git.
- Logs: store bounded database chunks and stream them through SignalR; object storage is unnecessary for Slice 1.
- Image digest: obtain it from structured helper output, not console-log parsing.
- Project status: derive it from current/last runs; do not duplicate it on `BuildProject`.
- External builders: prefer dedicated outbound build agents; do not start with AWS-specific EC2 provisioning.
