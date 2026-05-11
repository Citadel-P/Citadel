# Copilot Instructions

## Project Guidelines
- User prefers explicit pushback on weak design proposals; when a proposal is weak, explicitly challenge it and explain why instead of affirming by default.
- User prefers deterministic cross-platform hashing and centralized invariant enforcement for domain state transitions rather than relying on convention alone.
- For Git accounts, enforce domain invariants centrally: SSH transport requires SSH key authentication, and SSH key authentication cannot be used with non-SSH transport.
- In background jobs or long-running services, enqueue DB writes through `IDbWorkQueue` with a dedicated DB work item so SQLite locks are acquired and released quickly.
- User prefers alert SignalR notifications to be enqueued through `INotificationQueue` after the DB commit instead of awaiting them inline in the DB work item, to release SQLite locks quickly.
- When adding endpoints, include their request/response models in `src\Citadel.WebApi\Routes\Endpoints\STJContext\ApplicationJsonContext.cs` because the project uses source-generated JSON serialization. SignalR payloads must also be added to the configured SignalR serialization context.
- User prefers bulk acknowledge/resolve for alert events: the API should accept a list of `IEnumerable<Guid>` for command ids rather than single-id operations for manual alert workflows. For alert event bulk commands, fetch all requested alert events in one repository call and persist them with a bulk update instead of per-id loops, following patterns like `AlertRuleRepository.GetAllAsync` and `ContainerRepository.BulkUpsertAsync`.
- For domain and batch operations, prefer explicit result-based flow over exception-driven control flow. Do not use exceptions for expected domain validation branches such as state checks.
- On hot paths, prefer allocation-aware handling over unconditional LINQ materialization; avoid patterns like unconditional `Distinct().ToArray()` when a small-count fast path can skip extra allocations.
- User prefers keeping a single allocation in repository fetch paths; do not move handler-side list allocations into the repository with constructs like `[.. rows.Select(...)]`. Prefer returning `IEnumerable` when that preserves a single allocation.
- Domain JSON source-generation contexts should live in `DomainJsonContext.cs` rather than separate files.
- For Stack persistence, treat Stack as the aggregate root and do not expose a separate StackRelease repository; child StackRelease persistence should be handled through StackRepository.
- Do not modify the validation pipeline when fixing tests in this codebase; prefer updating tests to match handler behavior instead.
- Follow the established resource update pattern by splitting updates into Patch, PatchMetadata, and Rename operations, and mirror that pattern consistently in tests.
- Prefer hoisting shared ActorScope CTEs once per query; for list handlers, prefer DB-side filtering using shared authorization CTEs instead of in-memory filtering. When handling authorization for list handlers, distinguish the global permission attribute from DB-side filtering — treat attributes as expressing global (non-resource) checks and do not assume an attribute change fixes DB-side filtering behaviors unless the global permission check semantics are actually changed. Avoid `OR EXISTS` in authorized repository queries when a `UNION`-based authorization set is cleaner and more scalable. Prefer `JOINs` over `IN (SELECT ...)` for actor-scope authorization checks, and simplify resource name resolution by using a unified lookup projection (for example a SQL view or UNION ALL lookup) instead of multiple conditional joins per resource type.
- For batch container commands, avoid per-item authorization loops; perform batch access checks in a single authorization call or CTE that validates all container IDs together.
- When handling stale container stats batches, prefer using `IPlatformContainerCache` to validate container existence instead of querying the database again.

## Caching Guidelines
- For permission caching, cache only global checks where `resourceId` is null; do not cache resource-level permission checks. Prefer sliding expiration for the global permission cache.
- Preserve permission attributes when they are intended to enforce global (non-resource) permissions; align these attributes with the global permission cache policy.
