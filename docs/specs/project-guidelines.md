# Project Guidelines

## Purpose

This file holds project-wide guidance that does not belong to a specific domain spec.

## Engineering Preferences

- Challenge weak design proposals explicitly and explain why instead of affirming by default.
- Prefer deterministic cross-platform hashing and centralized invariant enforcement for domain state transitions.
- For Git accounts, enforce domain invariants centrally: SSH transport requires SSH key authentication, and SSH key authentication cannot be used with non-SSH transport.
- In background jobs or long-running services, enqueue DB writes through `IDbWorkQueue` with a dedicated DB work item so SQLite locks are acquired and released quickly.
- Alert SignalR notifications should be enqueued through `INotificationQueue` after the DB commit instead of awaited inline in the DB work item.
- When adding endpoints, include request and response models in `src\Citadel.WebApi\Routes\Endpoints\STJContext\ApplicationJsonContext.cs` because the project uses source-generated JSON serialization. SignalR payloads must also be added to the configured SignalR serialization context.
- Handle Docker lookup intents, such as Image, Volume, and Network, in the handler layer. Keep controllers thin and delegate lookup and intent logic to handlers.
- For alert event bulk commands, fetch all requested alert events in one repository call and persist them with a bulk update instead of per-id loops, following patterns like `AlertRuleRepository.GetAllAsync` and `ContainerRepository.BulkUpsertAsync`.
- For domain and batch operations, prefer explicit result-based flow over exception-driven control flow. Do not use exceptions for expected domain validation branches such as state checks.
- On hot paths, prefer allocation-aware handling over unconditional LINQ materialization.
- Preserve single-allocation repository fetch paths. Do not move handler-side list allocations into repositories with constructs like `[.. rows.Select(...)]`; prefer returning `IEnumerable` when that preserves a single allocation.
- Domain JSON source-generation contexts should live in `DomainJsonContext.cs` rather than separate files.
- For Stack persistence, treat Stack as the aggregate root and do not expose a separate StackRelease repository; child StackRelease persistence should be handled through `StackRepository`.
- Do not modify the validation pipeline when fixing tests; prefer updating tests to match handler behavior instead.
- Follow the established resource update pattern by splitting updates into Patch, PatchMetadata, and Rename operations, and mirror that pattern consistently in tests.
- Prefer hoisting shared ActorScope CTEs once per query. For list handlers, prefer DB-side filtering using shared authorization CTEs instead of in-memory filtering.
- When handling authorization for list handlers, distinguish the global permission attribute from DB-side filtering. Treat attributes as expressing global, non-resource checks.
- Avoid `OR EXISTS` in authorized repository queries when a `UNION`-based authorization set is cleaner and more scalable.
- Prefer `JOIN`s over `IN (SELECT ...)` for actor-scope authorization checks.
- Simplify resource name resolution by using a unified lookup projection, for example a SQL view or `UNION ALL` lookup, instead of multiple conditional joins per resource type.
- For batch container commands, avoid per-item authorization loops. Perform batch access checks in a single authorization call or CTE that validates all container IDs together.
- When handling stale container stats batches, prefer using `IPlatformContainerCache` to validate container existence instead of querying the database again.

## Database Migrations

- Do not hand-write new migration scripts under `src\Citadel.Infrastructure\Scripts`.
- Model schema changes in `src\Citadel.Infrastructure.Migrations\EntityFramework\ApplicationDbContext.cs`.
- Generate EF migrations under `src\Citadel.Infrastructure.Migrations\Migrations`.
- Regenerate the SQL script from `src\Citadel.Infrastructure.Migrations` with:

```powershell
dotnet ef migrations script -o "../Citadel.Infrastructure/Scripts/script0001.sql"
```

- Keep `script0001.sql` as generated output. Do not add follow-up files such as `script0002.sql` unless the migration strategy is intentionally changed.

## Release Workflow

### Development / Preview Builds

- Normal builds automatically produce preview versions, for example `1.0.5-preview-gabc123`.
- Version shape:
  - `1.0` is the current release train.
  - `5` is the git commit height.
  - `preview` is the prerelease label.
  - `gabc123` is the git commit hash for traceability.

### Start A New Development Version Cycle

- When ready to start a new version cycle, update `version.json`:

```json
{
  "version": "1.1"
}
```

- Commit it:

```powershell
git add version.json
git commit -m "Start 1.1 development"
```

- Preview builds then become `1.1.1-preview-gHASH`.

### Creating A Stable Release

- Prepare `main`:

```powershell
git checkout main
git pull
```

- Create and push a release tag:

```powershell
git tag v1.0.0
git push origin v1.0.0
```

- NBGV detects stable releases when the current ref matches `^refs/tags/v\d+\.\d+\.\d+$`.

## Caching Guidelines

- For permission caching, cache only global checks where `resourceId` is null; do not cache resource-level permission checks.
- Prefer sliding expiration for the global permission cache.
- Preserve permission attributes when they are intended to enforce global, non-resource permissions; align these attributes with the global permission cache policy.

## Build image

```powershell
docker build -t citadel.dev -f src/Citadel.WebApi/Dockerfile .
```

## Run the image: 

```powershell
docker run -d -p 8000:8000 -p 8001:8001 -v "citadel_data:/app/data" -v "/var/run/docker.sock:/var/run/docker.sock" --name citadel.dev citadel.dev
```
