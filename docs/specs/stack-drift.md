# Stack Drift

## Purpose

Stack drift compares the desired state of a deployed manual stack with the Docker runtime state. It is meant to identify changes that happened outside the normal Citadel stack apply flow, then optionally reconcile safe drift.

This feature currently applies only to manual stacks created from the web editor. Git-backed stacks are out of scope.

## Main Files

- `src/Citadel.Application/Services/StackDriftServices.cs`
- `src/Citadel.Application/TaskJobs/StackDriftMonitorJob.cs`
- `src/Citadel.FrontEnd/src/features/stacks/actions.tsx`
- `src/Citadel.FrontEnd/src/features/stacks/form/form.tsx`
- `src/Citadel.FrontEnd/src/features/stacks/form/index.tsx`
- `test/Citadel.Tests.Unit/Application/Features/Stacks/StackDriftTests.cs`

## Drift Policy

Stacks carry a `StackDriftPolicy`.

Supported modes:

- `Disabled`: drift checks are disabled.
- `DetectOnly`: drift can be detected and surfaced, but reconciliation is not allowed.
- `AutoFix`: eligible drift can be reconciled automatically or by the sync action.

Additional policy flags:

- `AlertOnDrift`: emits `StackDriftDetected` alerts when drift is found.
- `MarkDegraded`: marks the stack release as `Degraded` while unresolved drift exists.
- `AutoStartStoppedContainers`: allows stopped containers to be started.
- `AutoResumePausedContainers`: allows paused containers to be resumed.
- `RemoveExtraContainers`: allows extra owned containers to be removed.

When `mode` is `Disabled`, alerting and fix flags should be treated as disabled.

## Desired State

Desired state is parsed from the stack's current manual compose file.

The parser extracts:

- service name
- image reference
- expected Citadel service hash

The expected hash is generated from the compose service definition after Citadel label injection. This lets drift detection catch service configuration changes even when the service name still exists.

## Runtime State

Runtime state is read from Docker containers owned by the stack.

Containers are selected using the Citadel stack ownership filter:

- compose project label
- Citadel managed label
- Citadel stack id label

Each runtime container is inspected so drift can use labels and state details that may not exist in the list response.

## Drift Types

Detected drift types:

- `MissingContainer`: a compose service has no owned runtime container.
- `ExtraContainer`: an owned runtime container does not map to a desired service.
- `ContainerStopped`: an owned service container is stopped, dead, exited, or offline.
- `ContainerPaused`: an owned service container is paused.
- `ContainerUnhealthy`: an owned service container has a non-healthy health status.
- `ConfigHashMismatch`: an owned service container does not match the expected service hash.

`ImageMismatch` exists in the model but the current implementation does not compare desired image strings against runtime image strings.

## Eligibility

Drift checks return no drift for:

- non-web-editor stacks
- stacks with release status `Stopped`
- stacks with release status `Paused`

The monitor job checks stacks returned by `GetDriftMonitorStacksAsync`.

The UI enables drift query/sync only for stacks in `Healthy` or `Degraded` state and when drift mode is not `Disabled`.

## Reconciliation

Reconciliation is handled by `StackReconciler`.

It is allowed only when:

- the stack is not currently processing
- the release is not `Applying` or `Pending`
- `StackDriftPolicy.Mode == AutoFix`
- at least one drift item is allowed by the policy flags

Reconciliation can:

- start stopped containers
- resume paused containers
- remove extra containers

Structural drift is not auto-fixed:

- missing containers
- image mismatch
- config hash mismatch

Structural drift should direct the user to reapply the stack.

## Processing State

Reconciliation marks the stack as processing before applying actions and releases processing afterward.

Processing follows the same concurrency pattern used by stack state changes:

- load the stack
- capture previous release status
- call `MarkProcessing(actorId)`
- persist with `UpdateProcessingAsync(..., checkRowVersion: true, ...)`
- notify stack subscribers
- on success release to `Healthy`, `Degraded`, or previous status
- on failure release to previous status

The actor is the current user when available, otherwise `Constants.SystemId`.

## Activities And Alerts

The monitor persists `StackDriftDetected` activity events when the drift fingerprint changes.

If drift is later resolved and the stack was degraded, it records `StackDriftResolved`.

If reconciliation actually attempts actions, it records `StackReconciliationAttempted`.

Alerts:

- `StackDriftDetected` is emitted only when `AlertOnDrift` is enabled.
- Drift fingerprints are used to avoid repeatedly recording the same drift activity.

## UI Behavior

Stack actions:

- the sync action is available only for a single selected stack
- the stack must be `Healthy` or `Degraded`
- drift mode must not be `Disabled`
- the stack must not be processing
- a drift report must exist
- the report must have drift
- the report must not have structural drift
- at least one drift item must be actionable under the current drift policy

Stack detail containers tab:

- shows drift status above the containers table
- shows "No drift detected" when runtime matches desired state
- shows drift details when drift exists
- shows "Reapply required" for structural drift
- reconciliation is triggered from the stack action bar Sync action, not from the drift warning panel

The containers tab itself is disabled for stacks in `Created` state.

## Tests

Current test coverage should include:

- desired state parsing
- drift detection for missing, extra, stopped, paused, unhealthy, and config hash mismatch cases
- no drift for stopped/paused stacks
- monitor activity deduplication by fingerprint
- degraded status updates
- reconciliation processing state and activity recording
- sync failure behavior when auto-fix is not enabled

## Follow-Up Notes

If image-level drift is needed later, implement real image comparison before relying on `ImageMismatch`.

If Git stacks are added later, create a separate desired state provider instead of expanding the manual stack parser with Git-specific behavior.
