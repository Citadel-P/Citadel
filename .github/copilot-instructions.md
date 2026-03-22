# Copilot Instructions

## Project Guidelines
- User prefers explicit pushback on weak design proposals; when a proposal is weak, explicitly challenge it and explain why instead of affirming by default.
- User prefers deterministic cross-platform hashing and centralized invariant enforcement for domain state transitions rather than relying on convention alone.
- In background jobs or long-running services, enqueue DB writes through `IDbWorkQueue` with a dedicated DB work item so SQLite locks are acquired and released quickly.
- User prefers alert SignalR notifications to be enqueued through `INotificationQueue` after the DB commit instead of awaiting them inline in the DB work item, to release SQLite locks quickly.
- When adding endpoints, include their request/response models in `src\Citadel.WebApi\Routes\Endpoints\STJContext\ApplicationJsonContext.cs` because the project uses source-generated JSON serialization. SignalR payloads must also be added to the configured SignalR serialization context.
- User prefers bulk acknowledge/resolve for alert events: the API should accept a list of `IEnumerable<Guid>` for command ids rather than single-id operations for manual alert workflows. For alert event bulk commands, fetch all requested alert events in one repository call and persist them with a bulk update instead of per-id loops, following patterns like `AlertRuleRepository.GetAllAsync` and `ContainerRepository.BulkUpsertAsync`.
- For domain and batch operations, prefer explicit result-based flow over exception-driven control flow. Do not use exceptions for expected domain validation branches such as state checks.
- On hot paths, prefer allocation-aware handling over unconditional LINQ materialization; avoid patterns like unconditional `Distinct().ToArray()` when a small-count fast path can skip extra allocations.
- User prefers keeping a single allocation in repository fetch paths; do not move handler-side list allocations into the repository with constructs like `[.. rows.Select(...)]`. Prefer returning `IEnumerable` when that preserves a single allocation.
