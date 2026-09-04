# Phase 6D–6F Stack orchestration report

## Outcome

The Rust Core now owns the Web Editor Stack lifecycle for Docker Standalone
and Docker Swarm Platforms: CRUD, duplicate drafts, Apply progress, releases,
rollback, deletion, Standalone state actions, Swarm preflight, Compose/Swarm
import, drift inspection and safe reconciliation, and the pure manual-image
update evaluation rules.

Apply is claimed transactionally before Docker I/O, is bounded to four
concurrent operations and fifteen minutes, and uses a bounded progress channel.
A lost browser does not cancel the claimed operation. Runtime acceptance is not
success: the result is observed and the release plus Activity are committed in
one transaction. Ambiguous timeout or transport loss remains `Applying` and is
revisited only after the live Apply bound has elapsed.

## Compatibility and safety

- Existing .NET Stack spec discriminators and PascalCase PostgreSQL JSON remain
  readable; HTTP remains camelCase and JSON Merge Patch distinguishes an
  omitted description from an explicit `null`.
- Standalone container labels and Swarm Service/task labels use Citadel's
  established ownership keys. Reserved user labels fail before Docker runs.
- Swarm Compose preflight is fail-closed and returns the existing
  `isCompatible`/`issues` contract, including bounded file count and bytes.
- Compose variables and internal Secrets are resolved only for execution;
  Secret values are redacted from progress and persistent evidence.
- Import fingerprints include authoritative service identity and runtime state.
  Import links the complete namespace in one transaction and performs no Docker
  mutation.
- A Stack deletion claims and validates the complete batch before runtime work.
  Local and signed Agent transports share the same application port and an
  ambiguous mutation is never retried.
- Standalone state commands claim the complete batch transactionally before
  Docker I/O. Successful commands persist the final release state and typed
  `StackStarted`/`StackStopped`/`StackPaused` evidence; definite failures
  restore only claims still owned by that operation. Ambiguous timeouts remain
  `Pending` and are reconciled from a stable runtime aggregate after the live
  operation window, separately from deletion recovery.
- Releases use the existing `{ "releases": [...] }` response envelope and the
  generated OpenAPI declares the same error responses and query parameters as
  the current frontend contract.

## .NET test mapping

| Existing .NET behavior | Rust proof |
| --- | --- |
| `StackTests`, release preparation and aggregate status | `citadel-stacks` unit tests plus PostgreSQL release/rollback tests |
| `SwarmStackComposeCompatibilityTests` | `compose` unit tests for supported input, empty resources, unsupported keys/values, nested structure, build bindings, portability warnings, multi-file merge and limits |
| `StackDriftTests` safe runtime drift cases | `service` unit tests for missing, extra, stopped, paused and intentionally stopped state |
| `ManualStackAutoUpdateTests` check selection and update baseline | `updates` unit tests for scheduled/on-demand policy, tagged/digest/build images, baseline and duplicate notification suppression |
| `StackCreateTests`, `StackPatchTests`, `StackViewTests`, `GetStackReleasesTests`, `RollbackStackTests` | real-PostgreSQL adapter test and Axum endpoint test |
| `StackLifecycleTests` and `StackApplyRecoveryTests` core claims | concurrent Apply rejection, healthy snapshot preservation, transactional completion/failure, delete claim recovery and superseded-operation guards in adapter/HTTP tests |
| `ChangeStackState` cancellation, batch claim and completion behavior | PostgreSQL and Axum state-action tests prove all-before-I/O claims, conflict exclusion, rollback to the previous status, persisted final state, typed Activity evidence and stable-only stale recovery |
| `SwarmStackApplyEndpointTests` | Axum test verifies Swarm dispatch, exact ownership labels, undefined required variable rejection and persisted success |
| `SwarmStackImportEndpointTests` and import-draft factory ownership cases | Axum and PostgreSQL tests verify draft/validation, authoritative fingerprints, orphan ownership lookup, atomic namespace linking and no runtime mutation |
| Local/Agent Docker execution | signed Agent transport test and ignored Linux real-Docker Compose lifecycle test |

The source-backed Git materializer, Git/webhook update execution, registry
digest lookup, build-backed images, external Secret providers, retained Swarm
Secret/Config rollback materialization, and Edge Agent mutations depend on the
Phase 7 or Phase 10 external-execution transports. Their .NET tests are mapped
there rather than replaced by mocks in this gate. The update evaluator is
implemented in Phase 6; network lookup and automatic Apply are Phase 7.

## Gate

Run from the repository root:

```powershell
.\rust\scripts\Test-Phase6Stacks.ps1
```

The gate creates disposable PostgreSQL and network fixtures, runs the Stack
domain, signed-Agent, persistence and HTTP suites, executes a real Docker
Compose Apply/ownership/cleanup lifecycle, and verifies OpenAPI and generated
Docker contracts.

To include the native Swarm lifecycle, first initialize a disposable test
daemon as a Swarm manager and run:

```powershell
.\rust\scripts\Test-Phase6Stacks.ps1 -RunSwarmLifecycle
```

The gate deliberately refuses to initialize or alter the developer's Swarm.
