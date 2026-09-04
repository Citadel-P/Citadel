# Phase 6B Deployment Apply report

Date: 2026-09-03

## Outcome

Phase 6B is implemented for Docker Standalone Deployments over Local and
signed Agent transports. Rust now owns `POST /api/v1/deployments/apply`, its
streaming response, operation claim, referenced binding resolution, Docker
image preparation, Container creation/start observation, transactional
completion, realtime invalidation, and restart recovery.

This slice does not make Deployment a Swarm Service. Build-backed images,
external Secret providers, and Edge Agent mutation routing remain explicit
later-phase boundaries and return actionable errors.

## Execution and ownership

1. The HTTP boundary authenticates the actor and requires Deployment Execute
   plus the Apply specific permission.
2. PostgreSQL locks the Deployment, verifies its ACL, Docker Standalone
   Platform, Platform ownership, and idle operation state, then atomically
   claims Apply.
3. A four-slot application semaphore bounds concurrent detached Apply work.
   Waiting for capacity occurs before the database claim.
4. The operation verifies the Platform is Online, prepares a Local image or
   pulls an External image, resolves only referenced variables and Secrets,
   and creates the Container with Citadel ownership labels.
5. A Docker health check, when present, takes precedence over the `running`
   flag. Without one, normal Container state determines readiness.
6. PostgreSQL atomically links/upserts the Container projection, updates the
   Deployment and external digest, releases the claim, and writes a typed
   `DeploymentApplied` Activity. Secret snapshots contain only `********`.
7. The progress receiver is bounded and may disappear without cancelling the
   claimed operation. Shutdown and operation timeouts leave an ambiguous claim
   intact. A single bounded worker scans at most ten stale claims each minute,
   observes the ownership label, and completes or fails each claim.

Runtime mutations are never automatically retried after an ambiguous response.
If Docker Start returns an error after Container creation, Rust observes the
owned Container instead of issuing a duplicate Start.

## .NET characterization mapping

| Existing .NET behavior | Rust proof |
| --- | --- |
| `Apply_Deployment_WithLocalImage_ReturnsSuccess` | The Axum/PostgreSQL lifecycle streams Apply completion and persists Healthy/Idle; the Linux acceptance test creates, observes, and deletes a real Container. |
| `Apply_Deployment_WithExternalImage_PullsImageThenApplies` | The Unix-socket transport test verifies versioned image pull and registry-auth headers; runtime mapping verifies repository/digest selection. |
| `Apply_Deployment_WithRecreate_DeletesExistingContainerFirst` | Service/runtime tests preserve delete-before-create semantics when `recreate` is true. |
| not-found, Swarm Platform, Platform mismatch, and concurrent operation cases | HTTP and PostgreSQL tests exercise pre-mutation authorization/claim rejection; no runtime connector opens before the claim is valid. |
| disconnected Platform | Apply remains a successful HTTP stream, then records Failed/Idle and failure Activity without opening a Docker mutation, matching the existing progress-endpoint behavior. |
| connector failure and non-running Container cases | Unit and PostgreSQL tests persist Failed/Idle plus failure Activity and preserve a known Container link. |
| `ApplyAsync_Should_Inject_And_Snapshot_Only_Configured_Citadel_Entries` | Unit and PostgreSQL tests verify referenced-only injection, Deployment-over-Global precedence, encrypted internal Secret resolution, and masked evidence. |
| request cancellation/recovery behavior | Unit tests drop the progress receiver, force operation timeout, and reconcile both confirmed-running and unconfirmed stale outcomes. |
| Local/Agent transport equivalence | Docker Unix-socket tests verify create/start/pull calls; the Agent fixture verifies the signed Deployment protobuf without retrying ambiguous mutations. |
| generated public contract | OpenAPI generation/check exposes the existing camelCase request and `DeploymentStreamItem[]` response at the existing path. |

The current .NET acceptance project has no isolated Deployment Apply candidate
scenario. The Rust gate therefore adds the missing real Docker acceptance at
the adapter boundary and retains the real Axum/PostgreSQL lifecycle test.

## Safety and resource bounds

- four concurrent Apply operations;
- 30-second capacity wait before returning conflict;
- 32-item non-blocking progress channel;
- ten-minute operation timeout;
- 30-second Container readiness/health timeout;
- at most ten stale claims per reconciliation pass;
- one-minute reconciliation interval with skipped missed ticks;
- one-megabyte maximum Docker stream item and bounded HTTP error bodies;
- no runtime retry after ambiguous pull, create, start, or Agent mutation.

## Verification

Run from the repository root:

```powershell
.\rust\scripts\Test-Phase6BDeploymentApply.ps1
```

The disposable gate covers Deployment unit tests, Linux Unix-socket Docker
transport, signed Agent transport, real PostgreSQL claims/completion/failure,
binding precedence and Secret masking, a real local Docker Container lifecycle,
the authorized Axum endpoint, and deterministic OpenAPI/Docker generation. It
removes its PostgreSQL fixture, test Container, and Docker network afterwards.

Passed locally on 2026-09-03:

- `Test-Phase6BDeploymentApply.ps1`, including the disposable PostgreSQL and
  real Docker fixtures;
- `cargo fmt --all -- --check`;
- workspace-wide Clippy with all targets and warnings denied;
- the complete non-ignored Rust workspace suite with one build job;
- frontend `npm run test:run` (100 files, 340 tests);
- deterministic OpenAPI and Docker subset checks (169 full and 117 public
  HTTP contracts).

## Next slice

Phase 6C owns managed Swarm Service desired state, durable Apply/scale/update,
rollout success versus Docker API acceptance, progress, Activities, and
reconciliation. Deployment container actions, adoption, update checks, logs,
terminal, and statistics remain separately mapped slices; they are not implied
by Deployment Apply.
