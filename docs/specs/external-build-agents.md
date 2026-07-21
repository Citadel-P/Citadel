# Citadel External Build Agents - Implementation Specification

This specification extends the existing Citadel build system with external build execution on disposable cloud compute.

The feature must be incremental. Existing local, regular-agent, and edge-agent platform builds must continue to work unchanged.

## 1. Purpose

Add first-class support for **Build Pools** backed by external build agents.

V1 focuses on AWS EC2 ephemeral builders:

```text
one BuildRun
-> one BuildAgentLease
-> one EC2 instance
-> one Citadel Agent container in builder mode
-> one Docker build and push
-> instance termination
```

No warm fleet, no shared runners, no multi-run worker reuse.

## 2. Product Goal

Users should be able to run isolated, disposable Docker builds near their infrastructure without maintaining a CI runner fleet.

The existing build product surface stays the same:

- Git repositories and branches
- Dockerfile builds
- Citadel secrets as BuildKit secrets
- registries and image tags
- webhooks and path filters
- live logs and persisted logs
- build run history
- activities
- deployment and stack build-image consumers
- `Redeploy On Build`

The only new project-level choice is where the build executes.

## 3. Non-Goals

V1 does not implement:

- managed build services such as AWS CodeBuild
- Kubernetes runners
- Azure or GCP providers
- warm or shared build workers
- multiple runs per worker
- arbitrary shell pipelines
- visual CI pipelines
- Docker Compose build orchestration
- multi-platform builds
- image scanning
- SBOM, provenance, or signing
- registry promotion workflows
- object-storage source bundles
- Core SSH orchestration into the builder instance
- a new Citadel worker project, executable, or Docker image

## 4. Product Model

### 4.1 Current Model

Citadel currently has:

- `BuildProject`: reusable build configuration
- `BuildRun`: one execution attempt
- `BuildRunLogEntry`: persisted logs
- Docker execution through a selected `PlatformId`
- platform connectors: `Local`, `Agent`, `EdgeAgent`
- Citadel-backed BuildKit secrets
- registry push
- webhooks
- deployment and stack consumers
- retention through the existing build cleanup flow

### 4.2 New Model

Add a resource:

```text
Build Pool
```

The UI should use **Build Pool**, not **External Agent Pool**. Citadel already has platform agents and edge agents; the build pool should not look like another infrastructure platform connector.

Build project builder selector:

```text
Builder
- Docker Platform
- Build Pool
```

`Docker Platform` uses the current `PlatformId` path.

`Build Pool` uses an external build pool and runs on disposable external compute.

Suggested navigation:

```text
Builds
- Projects
- Runs
- Build Pools
```

## 5. V1 Lifecycle Invariant

V1 is strict:

- AWS EC2 only
- ephemeral worker only
- one worker per run
- one run per worker
- one EC2 instance per lease
- Git-backed build projects only
- single architecture per pool
- user-provided compatible AMI
- outbound bidirectional gRPC
- no object-storage source fallback
- no SSH orchestration
- no warm pool
- no worker reuse

`MaxActiveBuilders` controls how many EC2 instances the pool may run concurrently.

If the pool is at capacity, additional runs stay queued until capacity is available or the queue timeout expires.

## 6. Existing Agent Image And Builder Mode

### 6.1 No New Project Or Image

Do not add a new `CitadelBuilder`, `Citadel.BuildWorker`, or separate worker Docker image in V1.

Extend the existing Citadel Agent project and image used for regular agent and edge-agent deployments.

Add:

```text
CITADEL_AGENT_MODE=builder
```

Builder mode starts only build-worker services.

It must not:

- register as a managed platform
- expose platform management endpoints
- start platform synchronization
- start Docker daemon event synchronization
- accept generic Docker management commands
- run edge-agent platform command routing

### 6.2 Mode-Specific Services

Agent runtime modes should have explicit startup paths:

```text
agent    -> inbound platform-management services
edge     -> outbound edge-agent platform-management session
builder  -> outbound build lease session
```

Shared code is expected:

- configuration binding
- logging
- Docker client services
- stream services
- registry auth helpers
- BuildKit secret helpers
- gRPC transport infrastructure
- version and capability reporting

Builder mode should be a small service graph inside the existing agent binary, not a parallel implementation.

### 6.3 Docker Access

The builder runs Docker builds on the external host Docker daemon.

The EC2 bootstrap should start the existing Citadel Agent image in builder mode with Docker socket access, for example:

```text
docker run --rm \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e CITADEL_AGENT_MODE=builder \
  -e CITADEL_CORE_URL=... \
  -e CITADEL_BUILD_LEASE_ID=... \
  -e CITADEL_BUILD_BOOTSTRAP_TOKEN=... \
  citadel-agent-image
```

The exact command is generated by Core.

## 7. AWS EC2 Build Pool

### 7.1 Pool Configuration

An AWS EC2 Build Pool contains:

- name
- description
- tags
- enabled state
- AWS region
- AWS credential secret or assume-role configuration
- instance type, required
- architecture: `Amd64` or `Arm64`
- AMI ID
- root volume size in GB
- subnet ID
- security group IDs
- IAM instance profile name or ARN
- optional key pair name for user-managed debugging access
- assign public IP
- instance tags
- maximum active builders
- queue timeout
- provisioning timeout
- registration timeout
- heartbeat timeout
- cleanup timeout
- maximum instance lifetime
- failure retention minutes

Do not hardcode `c5.2xlarge` as a default. The UI may suggest instance types, but the user must choose one.

Operational controls such as concurrency and timeouts belong to the pool, not inside provider-specific JSON.

### 7.2 AMI Contract

V1 uses user-provided AMIs.

The configured AMI must contain:

- Docker Engine
- BuildKit/buildx support
- Git
- CA certificates
- a way to start the existing Citadel Agent image in builder mode

Citadel does not publish or manage AMIs in V1.

Pool validation should verify as much metadata as AWS exposes, but runtime tools are ultimately verified by real provisioning and worker registration results.

### 7.3 Builder Image

The builder container is launched from the existing Citadel Agent image.

The pool may allow an agent image override only if Citadel already supports configurable agent images in the agent or edge-agent setup flow.

The worker must report:

- agent version
- protocol version
- OS architecture
- Docker version
- BuildKit/buildx availability
- Git availability
- supported build capabilities

Core rejects assignment if required capabilities are missing.

### 7.4 Network Model

The model is outbound-only from the builder to Core.

Assigning a public IP is optional. It may be required when the selected subnet has no NAT gateway or private route allowing the worker to reach:

- Citadel Core
- Git provider
- target registry
- package endpoints required by the Dockerfile
- OS or container runtime dependencies

Do not describe this as "Core callbacks" to the instance. Core does not need inbound network access to the builder.

Distinguish these connectivity paths:

- Core to AWS control plane
- EC2 subnet egress to Citadel Core
- EC2 subnet egress to Git
- EC2 subnet egress to registry
- EC2 subnet egress to package mirrors

Core-side pool validation cannot fully prove worker egress. Real provisioning and worker diagnostics update operational health.

### 7.5 Required EC2 Tags

Every Citadel-managed EC2 instance must include protected tags:

```text
ManagedBy = Citadel
CitadelResource = BuildPool
CitadelBuildPoolId = ...
CitadelBuildRunId = ...
CitadelBuildLeaseId = ...
CitadelExpiresAt = ...
```

User-provided tags must not override Citadel tags.

### 7.6 Provisioning Idempotency

Use the lease id as the AWS provisioning idempotency token.

Retrying `RunInstances` for the same lease must not create duplicate builders.

## 8. Bootstrap Flow

Core creates a lease and a random one-time bootstrap token.

Only the token hash is stored.

The token is bound to:

- pool id
- lease id
- build run id

The EC2 bootstrap configuration contains only:

- Core endpoint
- pool id
- lease id
- build run id
- bootstrap token
- protocol and TLS metadata
- agent image reference if required

Do not put these values in EC2 user data:

- Git credentials
- registry credentials
- BuildKit secret values
- AWS credentials
- long-lived Citadel API tokens

The worker registers over TLS, Core atomically consumes the token, and Core establishes a lease-scoped worker session.

The bootstrap token can never be used again.

## 9. Agent Communication

### 9.1 Transport

Use one outbound bidirectional gRPC stream, following the same broad pattern as the edge-agent control channel.

Worker to Core:

- registration
- capabilities
- heartbeat
- phase changes
- ordered log batches
- terminal result
- cleanup acknowledgement

Core to Worker:

- run assignment
- cancellation
- shutdown

Do not expose registration, events, and completion as public REST endpoints.

### 9.2 Message Identity

Every protocol message includes:

- `ProtocolVersion`
- `LeaseId`
- `BuildRunId`
- `WorkerSessionId`
- `MessageSequence`

Log batches also include:

- `LogSequenceStart`
- log entries

Core must reject messages when:

- the lease is not active
- the worker session does not match
- the build run is terminal
- the message sequence is stale
- the worker reports another run
- the protocol version is unsupported
- required capabilities are missing

Ordered logs must be idempotent across reconnects. Duplicate log batches should not produce duplicate persisted logs.

## 10. Source Strategy

External builds are Git-only in V1.

When a Build Pool is selected:

- Core syncs or validates the configured Git repository
- Core resolves the exact commit SHA
- Core sends repository reference, branch, commit SHA, and short-lived Git credentials to the worker
- worker clones the repository directly
- worker checks out the exact resolved commit
- worker fails the run if the checked-out commit does not match the snapshot

Do not transfer large source archives through the agent command envelope.

Do not implement object-storage source bundles in V1.

If a future build source is not Git-backed, the UI must disable Build Pool selection and explain why.

## 11. Build Execution

The worker:

1. creates an owner-only workspace
2. checks out the resolved commit
3. validates context and Dockerfile paths
4. prepares temporary Docker config for registry auth
5. prepares BuildKit secret files with `0600` permissions
6. runs Docker build through the Docker Engine API or existing Docker build service
7. pushes the configured image references
8. reports digest and image references to Core
9. deletes temporary credentials and workspace

Do not pass secrets as command-line arguments.

The worker should reuse existing Docker stream reading, Docker auth, and BuildKit secret transport primitives from the Citadel contracts and Docker client libraries.

## 12. Secrets And Credentials

External builders need short-lived access to:

- Git credentials for the selected repository
- registry credentials for push
- Citadel BuildKit secrets selected by the build project

AWS provisioning credentials remain on Core and are never sent to the worker.

Rules:

- raw secret values are not stored in build project rows
- raw secret values are not stored in build run snapshots
- raw secret values are not stored in pool snapshots
- raw secret values are not stored in activities
- raw secret values are not stored in persisted logs
- raw secret values are not broadcast over SignalR
- run snapshots may store secret ids and BuildKit ids only
- Core resolves secret values at run time
- worker receives only secrets needed for the assigned run

Use defense in depth:

- worker redacts logs before sending
- Core redacts logs again before persistence and SignalR

Log redaction does not prevent deliberate exfiltration by a Dockerfile that has access to a BuildKit secret. Permission to edit or run a build that uses a secret is effectively permission to use that secret. Existing Citadel secret-binding authorization must be enforced before the run starts.

## 13. Domain Model

### 13.1 BuildAgentPool

Add domain entity:

```text
BuildAgentPool
```

Suggested fields:

- `Id`
- `Name`
- `NormalizedName`
- `Description`
- `Enabled`
- `Provider`
- `ProviderSpec`
- `MaxActiveBuilders`
- `QueueTimeoutSeconds`
- `ProvisioningTimeoutSeconds`
- `RegistrationTimeoutSeconds`
- `HeartbeatTimeoutSeconds`
- `CleanupTimeoutSeconds`
- `MaximumInstanceLifetimeSeconds`
- `FailureRetentionMinutes`
- `LastValidationStatus`
- `LastValidationMessage`
- `LastValidatedAt`
- `CreatedByActorId`
- `CreatedAt`
- `UpdatedAt`
- `ArchivedAt`
- `RowVersion`

Use source-generated serialization for provider specs.

### 13.2 AWS Provider Spec

Provider configuration must be strongly typed, not an arbitrary dictionary.

Suggested shape:

```csharp
abstract record BuildAgentPoolProviderSpec;

record AwsEc2BuildAgentPoolProviderSpec(
    string Region,
    string InstanceType,
    CpuArchitecture Architecture,
    string AmiId,
    int RootVolumeSizeGb,
    string SubnetId,
    IReadOnlyList<string> SecurityGroupIds,
    string? InstanceProfileName,
    bool AssignPublicIp,
    Guid? AwsCredentialSecretId,
    string? AssumeRoleArn,
    string? KeyPairName,
    IReadOnlyDictionary<string, string> Tags
) : BuildAgentPoolProviderSpec;
```

### 13.3 BuildAgentLease

Add domain entity:

```text
BuildAgentLease
```

Suggested fields:

- `Id`
- `BuildAgentPoolId`
- `BuildRunId`
- `Status`
- `CleanupStatus`
- `ProviderInstanceId`
- `RegistrationTokenHash`
- `AgentId`
- `AgentVersion`
- `ProtocolVersion`
- `WorkerSessionId`
- `Capabilities`
- `MessageSequence`
- `LastLogSequence`
- `ProvisioningStartedAt`
- `ProvisioningCompletedAt`
- `RegisteredAt`
- `AssignedAt`
- `LastHeartbeatAt`
- `CleanupStartedAt`
- `CleanupCompletedAt`
- `CompletedAt`
- `ErrorCode`
- `ErrorMessage`

### 13.4 Builder Selection On BuildProject

Add a discriminated builder model to API contracts:

```json
{
  "builder": {
    "$type": "Platform",
    "platformId": "019..."
  }
}
```

```json
{
  "builder": {
    "$type": "BuildAgentPool",
    "buildAgentPoolId": "019..."
  }
}
```

Persistence may use columns if it better matches current conventions:

```text
BuilderKind
PlatformId
BuildAgentPoolId
```

Validation invariants:

```text
BuilderKind = Platform
-> PlatformId is not null
-> BuildAgentPoolId is null

BuilderKind = BuildAgentPool
-> BuildAgentPoolId is not null
-> PlatformId is null
```

Existing build projects migrate to:

```text
BuilderKind = Platform
PlatformId = existing value
BuildAgentPoolId = null
```

### 13.5 BuildRun Snapshot

Add run snapshot fields:

- `BuilderKind`
- `PlatformSnapshot`, nullable for build-pool runs
- `BuildAgentPoolSnapshot`, nullable for platform runs
- `BuildAgentLeaseId`, nullable for platform runs
- `ProviderInstanceId`, nullable until provisioned
- `Architecture`
- `Phase`
- phase timing fields
- cleanup warning fields

The build run remains the user-facing source of truth for build result.

### 13.6 Persistence Constraints

Add database constraints:

- build project builder invariants
- one lease per build run
- one active lease per build run
- unique active provider instance id
- unique registration token hash
- positive timeout and capacity values
- bounded failure retention
- bounded maximum instance lifetime

Persistence must follow current Citadel conventions:

- DTOs in `src/Citadel.Infrastructure/Persistence/Dtos`
- mappers in `src/Citadel.Infrastructure/Persistence/Mappers`
- source-generated JSON serialization for AOT
- EF migrations through existing migration workflow
- no dynamic JSON serialization paths

## 14. State Model

### 14.1 BuildRun Status

Do not overload build status with infrastructure details.

Keep the current Citadel build statuses:

```text
Queued
Preparing
Running
Succeeded
Failed
TimedOut
Cancelled
Interrupted
```

`Queued`, `Preparing`, and `Running` remain active statuses.

Terminal statuses remain:

```text
Succeeded
Failed
TimedOut
Cancelled
Interrupted
```

### 14.2 BuildRun Phase

Add a separate phase for active external runs:

```text
WaitingForCapacity
Provisioning
WaitingForWorker
CheckingOutSource
Building
Pushing
Finalizing
```

The phase is nullable for platform runs and terminal runs.

The run sheet can display the phase while preserving the stable build status model.

### 14.3 Lease Status

Suggested lease statuses:

```text
Pending
Provisioning
WaitingForWorker
Registered
Assigned
Running
Completed
Failed
Cancelled
TimedOut
Interrupted
```

### 14.4 Cleanup Status

Cleanup is infrastructure state, not build result.

Suggested cleanup statuses:

```text
NotRequired
Pending
Running
Succeeded
Failed
```

If image build and push succeed but EC2 termination fails:

```text
BuildRun.Status = Succeeded
BuildAgentLease.CleanupStatus = Failed
```

The UI shows:

```text
Succeeded - builder cleanup requires attention
```

Do not mark a successful image build as failed because cloud cleanup initially failed.

## 15. Timeout Semantics

Use separate timeout categories:

- queue timeout
- provisioning timeout
- registration timeout
- build execution timeout
- heartbeat timeout
- cleanup timeout
- maximum instance lifetime

The existing build project timeout remains the build execution timeout.

Cancellation behavior by phase:

| Phase | Expected action |
| --- | --- |
| Waiting for capacity | mark cancelled; no instance exists |
| Provisioning | cancel lease and terminate if an instance appears |
| Waiting for worker | terminate instance |
| Checkout, build, push | send cancellation, then force terminate after grace period |
| Cleanup | keep build result; continue idempotent termination |

All terminal transitions must be idempotent.

Late success after cancellation must not overwrite a terminal cancelled status.

## 16. Scheduler And Reconciliation

Add:

```text
ExternalBuildReconciler
```

This is required because external builders are cost-incurring cloud resources. Generic database retention cleanup is not sufficient for provisioning, heartbeats, or orphaned EC2 instances.

Responsibilities:

- claim queued Build Pool runs transactionally
- enforce pool capacity
- create leases
- provision EC2 instances
- use provisioning idempotency
- detect provisioning timeout
- detect registration timeout
- detect heartbeat loss
- forward cancellation
- terminate instances
- retry failed cleanup
- reconcile active runs and leases after Core restart
- discover orphaned Citadel-managed EC2 instances by tags
- enforce maximum instance lifetime
- finalize interrupted or timed-out runs

The existing generic cleanup job may still remove old terminal build runs and logs according to retention rules.

### 16.1 Core Restart

On startup or reconciliation:

- inspect non-terminal leases
- query AWS for known instance IDs
- avoid provisioning a second instance for an existing lease
- reconnect to an existing worker when possible
- mark lost workers interrupted after heartbeat timeout
- terminate instances whose run is already terminal
- retry leases with failed cleanup

### 16.2 Orphan Discovery

Periodically query AWS for instances tagged:

```text
ManagedBy = Citadel
CitadelResource = BuildPool
```

Terminate an instance when:

- its lease does not exist
- its run is terminal and failure retention does not apply
- `CitadelExpiresAt` is exceeded
- maximum instance lifetime is exceeded
- it belongs to an archived or deleted invalid lease and is no longer active

Never terminate an instance solely from a user-provided tag.

## 17. Pool Enable, Disable, And Archive Semantics

### 17.1 Disabled Pool

When disabled:

- it cannot be selected on new or edited build projects
- new runs using it cannot be queued
- active runs continue
- existing project references remain valid for history and editing

### 17.2 Archived Pool

`DELETE` means soft archive.

When archived:

- it is hidden from normal selectors
- new runs are rejected
- active runs continue to terminal state
- historical run snapshots remain readable
- cleanup and reconciliation continue

## 18. Pool Validation

Add:

```text
POST /api/v1/build-agent-pools/{id}/test
```

V1 pool testing validates Core-side configuration:

- AWS credentials or role assumption
- region
- AMI existence
- architecture metadata when available
- subnet existence
- security group existence
- instance profile existence
- permissions to create, tag, inspect, and terminate instances
- permission to pass only the configured instance profile

The test cannot guarantee worker reachability to Core, Git, the registry, or package endpoints. Surface this limitation in the UI.

Pool operational health is also updated from real provisioning, registration, and cleanup results.

Suggested display statuses:

```text
NotTested
Ready
Invalid
Degraded
```

An idle ephemeral pool is not unhealthy because it has no connected workers.

## 19. API

Use existing route naming conventions. Conceptual public endpoints:

```text
GET    /api/v1/build-agent-pools
POST   /api/v1/build-agent-pools
GET    /api/v1/build-agent-pools/{id}
PATCH  /api/v1/build-agent-pools/{id}
DELETE /api/v1/build-agent-pools/{id}
POST   /api/v1/build-agent-pools/{id}/test
GET    /api/v1/build-agent-pools/{id}/leases
```

Build project inputs must use a discriminated builder object rather than two unrelated active identifiers.

Example platform builder:

```json
{
  "builder": {
    "$type": "Platform",
    "platformId": "019..."
  }
}
```

Example build pool builder:

```json
{
  "builder": {
    "$type": "BuildAgentPool",
    "buildAgentPoolId": "019..."
  }
}
```

Persist as columns if that better matches current repository and mapper patterns.

The internal builder protocol is the bidirectional gRPC stream described above. Do not expose builder registration and event flow as public REST endpoints.

## 20. Authorization

Add capabilities:

```text
BuildAgentPools.View
BuildAgentPools.Create
BuildAgentPools.Update
BuildAgentPools.Delete
BuildAgentPools.Test
BuildAgentPools.Use
```

Rules:

- `View` permits reading pool metadata and status
- `Use` permits assigning the pool to a build project
- `Create`, `Update`, `Delete`, and `Test` administer pools
- running an already configured build project requires the existing build run permission
- editing a project to use a different pool requires `BuildAgentPools.Use` on the selected pool
- pool provisioning credentials are resolved only by trusted Core services

Activity records must include the actor who created, updated, tested, disabled, archived, or assigned a pool.

Builder sessions are not user actors and must not receive normal application authorization tokens.

## 21. User Interface

### 21.1 Navigation

Use separate build project and build pool resource surfaces:

```text
Builds
- build project list
- build project detail
  - Config
  - Runs
  - Activity

Build Pools
- build pool list
- build pool detail
  - Config
  - Leases
  - Activity
```

Do not put Build Pools inside a single build project detail page. Build Pools are reusable infrastructure, closer to registries or platforms. Many build projects may reference the same pool.

Do not present a build pool as another infrastructure platform or another edge agent.

### 21.2 Build Project Form

The existing build project detail page keeps its tabs:

```text
Config
Runs
Activity
```

In the `Config` tab, replace the platform-only placement field with:

```text
Builder: Docker Platform | Build Pool
```

When `Docker Platform` is selected:

- show the existing platform selector
- preserve all current behavior
- this covers local, regular-agent, and edge-agent platform builds

When `Build Pool` is selected:

- require a Git-backed build source
- show the pool selector
- show provider, region, architecture, instance type, and capacity summary
- hide platform-only fields
- warn when the pool is disabled, archived, invalid, or degraded
- explain that each run creates a disposable cloud instance

Compact pool summary under the selector:

```text
AWS EC2 - eu-west-3 - amd64 - c7i.large - Ready - 0/3 active
```

Warnings should be inline and specific:

```text
This pool is degraded. New runs may fail during provisioning.
```

### 21.3 Build Pool Page

Build Pools are their own resource page with list, detail, config, leases, and activity.

#### Build Pool List

Show:

- name
- provider
- region
- architecture
- instance type
- status
- active leases
- queued runs
- last validation
- tags

Actions:

- add Build Pool
- test
- enable or disable
- archive

#### Build Pool Config Tab

Show and edit:

- name
- description
- tags
- enabled state
- provider
- region
- architecture
- instance type
- AMI ID
- network summary
- agent image
- maximum active builders
- current active leases
- queued runs
- last validation status
- recent provisioning failures
- recent registration failures
- recent cleanup failures
- recent build runs using the pool

Do not show persistent connected agents because V1 workers are ephemeral and run-bound.

#### Build Pool Leases Tab

Show current and recent leases:

- linked build run
- build project
- lease status
- cleanup status
- EC2 instance ID
- worker session
- queued, provisioned, registered, started, and completed times
- error code and message

The run link opens the existing build run sheet or the build project's Runs tab with that run selected.

#### Build Pool Activity Tab

Show normal resource activity:

- pool created
- pool updated
- pool tested
- pool enabled or disabled
- pool archived
- provisioning started
- builder registered
- builder lost
- cleanup failed
- instance terminated

### 21.4 Run Sheet

Keep the existing build run sheet as the primary execution UI.

For external builds, show current phase:

```text
Queued
Waiting for capacity
Provisioning builder
Waiting for builder agent
Preparing source
Building
Pushing
Finalizing
Succeeded / Failed / Cancelled / Timed out / Interrupted
```

Metadata should include:

- trigger and actor
- repository
- branch
- resolved commit
- Build Pool
- provider
- instance ID when available
- architecture
- registry
- image references
- digest
- queue duration
- provisioning duration
- registration duration
- checkout duration
- build duration
- push duration
- cleanup duration

Build output remains in normal build logs.

Show cleanup problems as infrastructure warnings without changing a successful build result.

## 22. Logs And Observability

Persist and stream stable log sources:

```text
system
agent
git
docker
stdout
stderr
```

Do not create activity events for each log line.

Add timing and metrics for:

- queue wait
- provisioning
- agent registration
- source checkout
- Docker build
- push
- cleanup
- total instance lifetime

Recommended activity events:

```text
BuildAgentPoolCreated
BuildAgentPoolUpdated
BuildAgentPoolDisabled
BuildAgentPoolEnabled
BuildAgentPoolArchived
BuildAgentPoolTested
ExternalBuildProvisioningStarted
ExternalBuildAgentRegistered
ExternalBuildAgentLost
ExternalBuildCleanupFailed
ExternalBuildInstanceTerminated
```

Reuse existing `BuildRunStarted`, `BuildRunSucceeded`, and `BuildRunFailed` activity where present instead of creating duplicate build-result events.

## 23. Webhooks, Deployments, And Stacks

External builds must use the existing build project trigger and consumer flows.

Webhook behavior remains unchanged:

- validate provider signature
- validate repository identity
- validate branch
- use changed paths or Git diff fallback
- reject duplicate active runs according to current rules

Successful external builds must update consumers exactly like platform builds:

- deployments with image source `Build`
- stacks with build-image bindings
- `Redeploy On Build`
- build and consumer activity records
- SignalR updates
- persisted run logs describing affected consumers

Do not create a separate external-build deployment or stack model.

## 24. Failure Handling

Use stable error codes in addition to user-facing messages.

Required cases:

```text
PoolDisabled
PoolArchived
PoolCapacityTimeout
PoolConfigurationInvalid
AwsAuthenticationFailed
AwsAuthorizationFailed
AwsProvisioningFailed
AwsInstanceNotFound
AgentRegistrationTimeout
AgentTokenExpired
AgentTokenReplay
AgentProtocolMismatch
AgentCapabilityMismatch
AgentHeartbeatLost
SourceCloneFailed
SourceCheckoutFailed
SourceCommitMismatch
BuildFailed
PushFailed
BuildTimedOut
BuildCancelled
CleanupFailed
MaximumInstanceLifetimeExceeded
```

Behavior:

- provisioning failure marks the run failed
- queue timeout marks the run timed out
- registration timeout marks the run timed out or failed with the specific code
- lost heartbeat marks the run interrupted unless cancellation already won the race
- checkout, build, or push failure marks the run failed
- cleanup failure is recorded on the lease and retried
- cleanup failure must not leave the build project in processing
- late terminal messages after cancellation must not overwrite terminal status
- all terminal transitions must be idempotent

## 25. Security Requirements

- all builder communication uses TLS
- bootstrap tokens are hashed at rest
- bootstrap tokens are short-lived and single-use
- tokens are bound to pool, lease, and run
- worker sessions are lease-scoped
- AWS provisioning credentials remain on Core
- AWS permissions use least privilege
- `iam:PassRole` is scoped to approved instance profiles
- IMDSv2 is required
- security groups do not require inbound access from Core
- failed builders terminate by default
- debugging retention is bounded by `FailureRetentionMinutes`
- maximum instance lifetime always overrides debugging retention
- secret values are redacted on the worker and Core
- secret values are not stored in snapshots, logs, or activities
- the worker receives only data needed for its assigned run
- the workspace and temporary credentials are deleted after execution
- Citadel Core does not SSH into the instance

## 26. Licensing Boundary

Do not deeply couple cloud provisioning code to licensing.

Expose the feature through the existing license and capability layer using a stable feature identifier, for example:

```text
ExternalBuildPools
```

Rules:

- license checks happen at public command boundaries and run queueing
- domain and provider services must not parse license files directly
- existing platform builds remain available when the external-build feature is unavailable
- historical pools and runs remain readable when the feature is disabled
- active external runs should be allowed to finish if the license changes during execution

If licensing for this feature is not enabled yet, keep the capability hook but default it according to current Citadel product policy.

## 27. Implementation Slices

### Slice 1: Builder Target Model

- add `BuilderKind`
- add discriminated builder request and response contracts
- add project and run snapshot fields
- add `BuildRunPhase` for active external build phases
- migrate existing projects to `Platform`
- add domain and database constraints
- preserve all existing platform build tests

### Slice 2: Build Pool And Lease Foundation

- add `BuildAgentPool`
- add strongly typed AWS provider spec
- add `BuildAgentLease`
- add repositories, DTOs, mappers, JSON source generation, and migration
- add CRUD, test endpoint, permissions, and activity events
- add explicit lease state-transition service

### Slice 3: Existing Agent Builder Mode

- add `builder` to the existing agent runtime mode
- add mode-specific DI and hosted-service startup
- ensure builder mode does not start platform or edge management services
- add builder-specific gRPC stream using existing transport infrastructure
- add registration, session, heartbeat, cancellation, sequence handling, and tests
- do not add a new project, executable, or Docker image

### Slice 4: Local End-To-End Harness

Before AWS integration, validate the full builder-mode protocol with a local test harness or locally launched agent container:

```text
queue
-> lease
-> builder-mode agent registration
-> assignment
-> Git checkout
-> Docker build
-> registry push
-> logs
-> result
-> cleanup
```

This slice proves protocol and build behavior independently of EC2.

### Slice 5: AWS EC2 Provisioner

- implement Core-side AWS provider
- validate pool configuration
- provision with lease idempotency token
- tag instances
- generate user data
- launch the existing agent image in builder mode
- capture instance ID
- terminate idempotently
- enforce IMDSv2
- scope `iam:PassRole`

### Slice 6: Reconciliation And Hardening

- add `ExternalBuildReconciler`
- enforce pool capacity
- add all timeout handling
- add Core restart reconciliation
- add lost-heartbeat handling
- add orphan discovery
- retry cleanup
- enforce maximum instance lifetime
- add cancellation race tests

### Slice 7: UI

- add Build Pools list, details, and form
- add pool validation action
- update build project builder selector
- enforce Git-only external source rule
- display external phases and timing metadata
- display cleanup warnings
- display active leases and recent failures

### Slice 8: Existing Integrations

- verify webhook-triggered external builds
- verify deployment consumers
- verify stack consumers
- verify SignalR updates
- verify build log persistence and retention
- verify activity events
- verify licensing capability hook

## 28. Required Tests

### 28.1 Compatibility

- local platform builds remain unchanged
- regular-agent platform builds remain unchanged
- edge-agent platform builds remain unchanged
- existing project rows migrate correctly

### 28.2 Agent Modes

- builder mode starts builder services only
- builder mode does not register a platform
- builder mode does not start platform synchronization
- existing edge mode behavior remains unchanged
- invalid mode fails startup clearly

### 28.3 Pool And Capacity

- disabled pool rejects new runs
- archived pool rejects new runs
- maximum capacity queues additional runs
- queue timeout reaches a terminal state
- concurrent reconciliation cannot claim one run twice

### 28.4 Registration

- valid one-time token registers once
- expired token is rejected
- replayed token is rejected
- token for another lease is rejected
- protocol mismatch is rejected
- incompatible architecture or capabilities are rejected
- replaced sessions cannot report events

### 28.5 Provisioning

- retrying provisioning does not create duplicate instances
- Core restart does not create a second instance for an active lease
- failed AWS provisioning terminates the run
- `iam:PassRole` is restricted
- Citadel tags are always applied
- custom tags cannot override Citadel tags

### 28.6 Build Execution

- exact resolved commit is checked out
- commit mismatch fails before build
- Git authentication works without persisting credentials
- BuildKit secrets are injected without persistence
- registry push returns image digest
- logs stream and persist in order
- duplicate log batches are deduplicated

### 28.7 Cancellation And Timeouts

- cancellation while queued creates no instance
- cancellation during provisioning terminates the eventual instance
- cancellation while waiting for registration terminates the instance
- cancellation during build stops the worker or force-terminates after grace period
- late success after cancellation cannot overwrite cancelled status
- heartbeat loss marks the run interrupted
- maximum instance lifetime always triggers termination

### 28.8 Cleanup

- successful build plus successful termination stays succeeded
- successful build plus failed termination stays succeeded with cleanup warning
- cleanup retries are idempotent
- orphaned Citadel instances are detected
- orphaned instances are terminated
- non-Citadel instances are never terminated
- debugging retention expires automatically

### 28.9 Secrets

- raw secrets do not appear in project rows
- raw secrets do not appear in run snapshots
- raw secrets do not appear in pool snapshots
- raw secrets do not appear in activities
- raw secrets do not appear in persisted or streamed logs
- AWS credentials are never sent to the builder

### 28.10 Integrations

- webhook can trigger external build
- successful external build updates deployments
- successful external build updates stack build bindings
- redeploy-on-build behavior remains unchanged
- retention cleans old terminal runs without interfering with active leases

## 29. Acceptance Criteria

The feature is complete when:

- no new agent executable project exists
- no new agent Docker image exists
- the existing Citadel Agent image supports `CITADEL_AGENT_MODE=builder`
- builder mode runs only build-related services
- existing local, regular-agent, and edge-agent platform builds pass unchanged
- a user can create and validate an AWS EC2 Build Pool
- a user can select the pool on a Git-backed build project
- one manual run provisions one EC2 instance
- the instance starts the existing agent image in builder mode
- the builder registers outbound with a one-time lease-bound token
- the builder checks out the exact run commit
- the builder builds and pushes the image
- logs and phases stream through the existing Citadel run UX
- the resulting digest is persisted
- the EC2 instance is terminated
- webhook-triggered runs use the same external path
- successful builds update deployments and stacks exactly like platform builds
- pool capacity prevents excess instances
- Core restart does not duplicate instances or lose cleanup responsibility
- orphaned Citadel-managed instances are detected and terminated
- provisioning, registration, checkout, build, push, cancellation, timeout, heartbeat loss, and cleanup failures are terminal or reconciled correctly
- no build project remains stuck in processing
- raw secret values are never persisted or broadcast
- all persistence and API JSON contracts are AOT-safe

## 30. Codex Implementation Guidance

Before changing code, inspect the current repository and identify:

- the existing Citadel Agent project and Docker image build
- the existing agent, edge-agent, and local connector configuration
- the current outbound edge-agent gRPC channel
- build project and build run domain models
- build execution abstractions
- BuildKit secret session implementation
- registry authentication transport
- build log persistence and SignalR streaming
- background reconciliation and cleanup conventions
- permission and licensing capability conventions
- AOT JSON source-generation contexts

Implement by extending existing patterns rather than introducing parallel abstractions.

Prefer small vertical changes that keep the solution compiling and existing build paths passing after each slice.

Do not redesign unrelated build, platform, agent, deployment, stack, secret, or registry models unless a concrete incompatibility is found and documented.
