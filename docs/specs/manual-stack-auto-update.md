# Manual Stack Auto-Update

## Purpose

Manual stack auto-update detects new image digests for services defined in a manual stack compose file. Depending on `ManualStack.UpdateBehavior`, Citadel either records update availability, sends alerts, redeploys the full stack, or redeploys only the affected services.

Git-backed stacks are out of scope for this spec.

## Main Files

- `src/Citadel.Application/TaskJobs/ManualStackAutoUpdateJob.cs`
- `src/Citadel.Application/Services/ImageScanScheduler.cs`
- `src/Citadel.Application/Services/StackComposeParser.cs`
- `src/Citadel.Application/Services/ApplyStackService.cs`
- `src/Citadel.Domain/Entities/Stacks/StackSpec.cs`
- `src/Citadel.Domain/Contracts/Resources/Stacks/StackApplyCommand.cs`
- `src/Citadel.FrontEnd/src/features/stacks/form/form.tsx`
- `src/Citadel.FrontEnd/src/features/stacks/form/index.tsx`
- `src/Citadel.FrontEnd/src/features/stacks/table.tsx`
- `test/Citadel.Tests.Unit/Application/Features/Stacks/ManualStackAutoUpdateTests.cs`

## Update Behavior

Manual stacks use `StackUpdateBehavior`.

Supported values:

- `Disabled`: do not scan stack service images.
- `Notify`: record update state and emit an update-available alert.
- `ServiceAutoDeploy`: pull updated images and redeploy only changed services.
- `StackAutoDeploy`: pull updated images and redeploy the full stack.

Deployment auto-update uses a different enum, `UpdateBehavior`. Do not reuse deployment update behavior for manual stacks.

## Image Scan Eligibility

`ImageScanScheduler.LoadManualStackChecksAsync` creates checks only when:

- the stack source is `WebEditor`
- the current release spec is `ManualStack`
- `UpdateBehavior` is not `Disabled`
- the manual stack has a non-empty `RegistryId`
- the stack is not processing
- the current release status is `Healthy` or `Degraded`
- the service image can be split into repository and tag

Pinned digest-only images or unsupported image strings are ignored because they cannot be checked as tag updates.

Manual stack image checks are merged into `LoadScanTasksAsync`, so the same image digest cache used by deployments is also used by stacks.

## Compose Parsing

`StackComposeParser` parses the manual compose file and extracts service metadata:

- service name
- image name
- expected service hash

The same parser is shared with stack drift detection.

## State Model

Manual stack update state is stored on the stack as `ManualStackUpdateState`.

The current implemented state shape is:

- `RecreateStackOnNewImageState`
- `AutoUpdateStates`
- one `ImageUpdateState` per service image check

Each service state tracks:

- service name
- image name
- current digest
- remote digest
- last checked time
- whether an update is available

When an update is detected and only notification is required, `CurrentDigest` remains unchanged and `UpdateAvailable` remains true.

When an auto-deploy succeeds, changed services update `CurrentDigest` to the remote digest and clear `UpdateAvailable`.

## Background Job

`ManualStackAutoUpdateJob` runs periodically every two hours with jitter.

For each stack:

1. load eligible image checks
2. group checks by stack
3. wait for the platform image scanner barrier
4. compare cached remote digest against recorded current digest
5. update stack update state
6. notify, auto-deploy, or mark failure based on update behavior

If the image digest cache has no entry for a check, that check is skipped for the current pass.

## Alerts

Alert events emitted by manual stack auto-update:

- `StackImageUpdateAvailable`: update detected and behavior is `Notify`
- `StackAutoUpdated`: full stack auto-deploy succeeded
- `StackAutoDeployFailed`: full stack auto-deploy failed
- `StackServiceAutoUpdated`: service-scoped auto-deploy succeeded
- `StackServiceAutoDeployFailed`: service-scoped auto-deploy failed

Alert payloads include the stack name and the affected service/image update items. Service-scoped failure alerts also include the affected service names and failure reason.

## Apply Behavior

Manual stack auto-update uses `IApplyStackService.ApplyAsync` with explicit operation options.

For auto-update:

- actor id is `Constants.SystemId`
- `pullImages` is `true`
- `serviceNames` is set only for `ServiceAutoDeploy`

For normal UI stack apply:

- actor id comes from `IUserContextAccessor`
- `pullImages` is `false`
- `serviceNames` is null

`StackApplyCommand` carries:

- `ServiceNames`
- `PullImages`

The Docker compose implementation should:

- pass `--pull always` only when `PullImages` is true
- apply only the named services for service-scoped updates
- skip destructive full-stack behavior for service-scoped updates

## UI Behavior

Stack form:

- exposes `StackUpdateBehavior` for manual stacks
- stores the selected behavior on `ManualStack.UpdateBehavior`

Stack table:

- includes an "Update Status" column similar to deployments
- shows `Unknown` when no stack update state exists
- shows `UpdateAvailable` when any service state has `updateAvailable`
- shows `UpToDate` when service states exist and none have updates
- hover details show checked time, service, image, current digest, and available digest

Stack detail:

- shows an "Image update available" notice when one or more service states have `updateAvailable`

## Tests

Current test coverage should include:

- parsing compose services and service hashes
- scheduler filtering eligible manual stacks
- ignoring disabled, stopped, not deployed, processing, missing registry, and unsupported image cases
- alert metadata for stack and service-scoped auto-update events
- auto-update behavior state transitions
- service-scoped auto-deploy calling stack apply with service names and `pullImages: true`

## Follow-Up Notes

Git stack auto-update should be implemented separately later. It should have its own source/provider logic and should not be mixed into manual stack compose parsing.

If more update strategies are added, keep stack update behavior separate from deployment update behavior unless the domain model is intentionally unified.
