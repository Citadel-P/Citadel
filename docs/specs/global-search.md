# Citadel Global Search

This is the MVP implementation specification for a permission-aware resource
search in the authenticated Citadel header.

Before implementing this specification:

1. Compare it with the current codebase and keep existing Citadel naming,
   routes, authorization, and generated API patterns.
2. Keep the endpoint thin and put search behavior in an Application query and
   Infrastructure persistence code.
3. Run relevant backend tests, OpenAPI generation, frontend API generation,
   and frontend build checks.
4. Report any conflict between this specification and the current resource
   model before broadening the scope.

---

## 1. Purpose

Add a global search popup that lets an authenticated user quickly find and
open Citadel resources they are allowed to read.

This feature is a resource navigator. It is not a full-text search engine or a
general command palette. It must remain small, fast, permission-aware, and
consistent with Citadel's existing UI, React Router routes, RBAC model, and
resource-level access rules.

## 2. Goals

- Make search available from every authenticated page.
- Show supported resource categories before a query is entered.
- Search supported resources after the user enters at least two characters.
- Group matches under the labels already used in Citadel navigation.
- Show enough secondary information to distinguish similarly named resources.
- Show a text status and semantic status indicator where a useful status exists.
- Let the user navigate independently to a result or an accessible parent.
- Never return resources outside the current user's effective read access.
- Use PostgreSQL and Dapper already present in Citadel; do not add a search
  service or external index.

## 3. Citadel Resource Model

The UI has eight categories, but Citadel has ten concrete searchable
`Hosting.Common.ResourceType` values. The API must preserve the concrete type
so authorization and navigation are unambiguous.

| UI category | Concrete resource type | UI label | List route | Details route |
| --- | --- | --- | --- | --- |
| Platforms | `Platform` | Platform | `/platforms` | `/platforms/edit/{id}` |
| Stacks | `Stack` | Stack | `/stacks` | `/stacks/edit/{id}` |
| Deployments | `Deployment` | Deployment | `/deployments` | `/deployments/edit/{id}` |
| Repositories | `GitRepository` | Repository | `/git-repos` | `/git-repos/edit/{id}` |
| Registries | `Registry` | Registry | `/registries` | `/registries/edit/{id}` |
| Automations | `AutomationAction` | Automation | `/automation` | `/automation/edit/{id}` |
| Backups | `BackupPolicy` | Backup policy | `/backup-policies` | `/backup-policies/edit/{id}` |
| Backups | `BackupRepository` | Backup repository | `/backup-repositories` | `/backup-repositories/edit/{id}` |
| Builds | `Build` | Build project | `/builds` | `/builds/edit/{id}` |
| Builds | `BuildAgentPool` | Build pool | `/build-pools` | `/build-pools/edit/{id}` |

The platform list is also the authenticated index route. Use `/platforms` in
search metadata so the route is explicit and remains consistent with other
resource paths.

Do not introduce generic domain or permission values named `Repository`,
`Automation`, `Backup`, or `Builds`. They are presentation categories, not
Citadel resource types.

The MVP does not search run, history, snapshot, container, volume, image,
network, user, team, role, alert, or configuration records.

## 4. Frontend Metadata

Create one typed metadata source for category order, labels, icons, and route
generation. Result rows and the empty-query category list must use this source.

Use a shape equivalent to:

```ts
type GlobalSearchCategory =
  | 'Platforms'
  | 'Stacks'
  | 'Deployments'
  | 'Repositories'
  | 'Registries'
  | 'Automations'
  | 'Backups'
  | 'Builds'

type GlobalSearchResourceType =
  | 'Platform'
  | 'Stack'
  | 'Deployment'
  | 'GitRepository'
  | 'Registry'
  | 'AutomationAction'
  | 'BackupPolicy'
  | 'BackupRepository'
  | 'Build'
  | 'BuildAgentPool'

type GlobalSearchMetadata = {
  category: GlobalSearchCategory
  label: string
  singularLabel: string
  icon: LucideIcon
  getListPath: () => string
  getDetailsPath: (id: string) => string
}
```

Use `CitadelIcons` from `src/Citadel.FrontEnd/src/lib/icons.tsx`. Do not send
icon names or frontend URLs from the backend.

## 5. Header Entry Point

Add the search control to
`src/Citadel.FrontEnd/src/layout/header.tsx`, between the compact breadcrumb
and the right-side alert/account controls.

The control must:

- display a search icon and `Search resources` on widths where it fits;
- collapse to an icon button on narrow widths;
- show the platform-appropriate `Ctrl+K` or `Cmd+K` hint;
- open on click or `Ctrl/Cmd+K`;
- prevent the browser's default shortcut behavior after handling the event;
- not handle an event already prevented by an editor, input, or another modal;
- expose an accessible name and keyboard focus state.

Only one global search popup may be open. Closing and reopening it starts with
an empty query unless product behavior is changed explicitly later.

## 6. Popup Behavior

Build the popup with the existing shadcn/cmdk components in
`src/Citadel.FrontEnd/src/components/ui/command.tsx`. Follow the sizing,
spacing, colors, focus treatment, and font styles already used by Citadel.

### 6.1 Empty Query

Show the eight categories in the order from the resource table. Selecting a
category closes the popup and navigates to that category's list route.

Backups and Builds remain single category rows even though each maps to two
concrete resource types.

### 6.2 Short Query

For a trimmed query shorter than two characters:

- do not call the API;
- continue to show the category list;
- do not show a no-results message.

### 6.3 Searching

Debounce input by 250 to 300 milliseconds using the existing
`src/Citadel.FrontEnd/src/hooks/useDebounce.ts` hook.

Use TanStack Query through the existing API hook pattern in
`src/Citadel.FrontEnd/src/lib/hooks.ts`. Preserve its `AbortSignal` propagation
so obsolete requests are cancelled. A response for an older query must never
replace a newer result set.

Keep previous results visible during a refetch and show a restrained loading
indicator. Do not replace the entire popup with a blocking spinner.

### 6.4 Results

Render non-empty groups in the category order from the resource table. Within
Backups and Builds, preserve the API's ranked item order across both concrete
types.

Each result row must show:

- the concrete resource icon;
- primary name;
- concrete singular label where the category contains multiple types;
- one useful secondary value when available;
- status text and semantic indicator when available;
- an accessible parent link when the API returns a parent.

The main row opens the result. The parent link is a separate control and must
stop propagation so it does not open the child.

Keyboard behavior:

- Up and Down move through selectable rows.
- Enter opens the selected result.
- Escape closes the popup.
- Focus returns to the search trigger after close.

### 6.5 No Results And Errors

For a completed valid query with no matches, show:

```text
No resources found for "<query>"
```

For a failed request, show a compact error state with a retry action. Do not
turn authorization-filtered empty results into an error, and do not reveal
that inaccessible matching resources exist.

## 7. Matching And Ranking

Search is case-insensitive. Trim the query before validation and matching.

Search primary names for every type. Secondary fields may improve discovery,
but they must be non-secret and inexpensive to query:

| Resource type | Required searchable fields | Suggested secondary display |
| --- | --- | --- |
| `Platform` | name | address or platform type |
| `Stack` | name | accessible platform name |
| `Deployment` | name | accessible platform name |
| `GitRepository` | name, URL | URL or default branch |
| `Registry` | name, URL | URL or registry type |
| `AutomationAction` | name | enabled state or latest run status |
| `BackupPolicy` | name | source type or latest run status |
| `BackupRepository` | name | repository type or repository status |
| `Build` | name | Git repository/branch or latest run status |
| `BuildAgentPool` | name | connection mode or validation status |

Never search or return credentials, tokens, keys, secret values, automation
code, default argument JSON, build arguments, environment values, or logs.

Rank matches in this order:

1. exact primary-name match;
2. primary-name prefix match;
3. primary-name substring match;
4. secondary-field prefix match;
5. secondary-field substring match.

Use a stable tie-breaker of normalized name, concrete resource type, then ID.
The same request against unchanged data must return the same order.

Escape `%`, `_`, and `\` before using the term in `ILIKE`, and use an explicit
`ESCAPE '\'` clause. All values must be Dapper parameters.

## 8. Limits

- Minimum trimmed query length: 2 characters.
- Maximum trimmed query length: 100 characters.
- Default `limitPerType`: 5.
- Maximum accepted `limitPerType`: 10.
- Maximum response items across all groups: 40.

The limit applies to each concrete resource type before the global cap. The
Backups and Builds presentation groups may therefore contain results from both
of their concrete types, still subject to the 40-item response cap.

Apply filtering, ranking, and limits in PostgreSQL. Do not load complete
authorized resource collections and filter them in the Application layer.

## 9. Status Semantics

Return a status label and semantic tone, not a CSS class. The frontend owns
the visual classes.

```ts
type SearchStatusTone =
  | 'Positive'
  | 'Negative'
  | 'Warning'
  | 'Info'
  | 'Neutral'
```

Use current resource state:

- `Platform`: `PlatformStatus`.
- `Stack`: current `StackReleaseStatus`.
- `Deployment`: `DeploymentStatus`.
- `GitRepository`: `GitReposStatus`.
- `Registry`: `RegistryStatus`.
- `AutomationAction`: active/latest `ActionRunStatus`, otherwise enabled state.
- `BackupPolicy`: active/latest `BackupRunStatus`, otherwise enabled state.
- `BackupRepository`: `BackupRepositoryStatus`.
- `Build`: active/latest `BuildRunStatus`, otherwise enabled state.
- `BuildAgentPool`: `BuildAgentPoolValidationStatus`.

`BuildAgentPoolView` currently exposes validation status, not a separate
persisted connection status. Do not label validation as live connectivity. A
future connection-state field may replace it when the domain model supports
that distinction.

Map current states consistently:

- Positive: Online, Healthy, Active, Ready, Succeeded;
- Negative: Offline, Failed, Invalid, TimedOut, Interrupted;
- Warning: Degraded, Deprecated, Pending, Queued, Paused, Applying,
  SucceededWithWarnings;
- Info: Created, Uninitialized, Preparing, Processing, Provisioning, Running,
  ApplyingRetention;
- Neutral: Unknown, NotTested, Stopped, Disabled, Cancelled, Rejected.

Extend the mapping when a current enum contains another value. Keep the text
visible so status is not communicated by color alone.

The search result UI and
`src/Citadel.FrontEnd/src/components/custom/state-indicator.tsx` must share
semantic status behavior. Refactor a common status-to-tone helper if needed;
do not maintain contradictory mappings.

## 10. Parent Resources

The optional parent provides context and independent navigation.

Supported parent relationships:

- `Stack` -> `Platform`;
- `Deployment` -> `Platform`;
- `BackupPolicy` -> its first-class source resource when the configured
  `BackupSourceType` and stored ID identify a navigable Citadel resource;
- `Build` -> `Platform` or `BuildAgentPool`, according to its builder
  configuration.

Do not invent a single parent for `BackupRepository`; repositories can have
execution-location configuration that is not an ownership relationship.

The user must have `PermissionLevel.Read` with `SpecificPermission.None` for
the parent resource. Parent access is checked independently from child access.
If parent access is missing, omit the complete parent object, including its ID,
name, and type. Child visibility must not imply parent visibility.

## 11. HTTP API

Add the authenticated endpoint:

```http
GET /api/v1/search?q=prod&types=Platform,Stack&limitPerType=5
```

Query parameters:

| Parameter | Required | Behavior |
| --- | --- | --- |
| `q` | yes | trimmed, 2 to 100 characters |
| `types` | no | comma-separated concrete search resource types |
| `limitPerType` | no | default 5, range 1 to 10 |

`types` accepts only the ten concrete resource types listed in section 3.
Unknown values return a validation problem response. An omitted value searches
all supported types.

Return `400` for invalid input, `401` for an unauthenticated request, and `200`
for a valid request even when every result is filtered by authorization.

### 11.1 Response

```json
{
  "query": "prod",
  "groups": [
    {
      "category": "Platforms",
      "items": [
        {
          "id": "70692ca6-9809-4703-8b4d-fdf7cf8506dc",
          "resourceType": "Platform",
          "name": "production-east",
          "secondaryText": "docker.example.internal",
          "status": {
            "label": "Online",
            "tone": "Positive"
          },
          "parent": null
        }
      ]
    },
    {
      "category": "Builds",
      "items": [
        {
          "id": "af4a99d1-34c4-4be9-903d-f12dd857c34d",
          "resourceType": "Build",
          "name": "production-api",
          "secondaryText": "main",
          "status": {
            "label": "Succeeded",
            "tone": "Positive"
          },
          "parent": {
            "id": "70692ca6-9809-4703-8b4d-fdf7cf8506dc",
            "resourceType": "Platform",
            "name": "production-east"
          }
        }
      ]
    }
  ]
}
```

Response rules:

- return only non-empty groups for a search query;
- use the eight presentation category names exactly as documented;
- include the concrete `resourceType` on every item;
- use `null` or omit optional status, secondary text, and parent consistently;
- never include edit URLs, icon names, permission details, or match counts for
  inaccessible data;
- do not expose the ranking score.

## 12. Authorization

Every result requires effective:

```text
ResourceType = the concrete result type
PermissionLevel = Read
SpecificPermission = None
```

Effective access includes:

- permissions assigned directly to the user's actor;
- permissions inherited from enabled team actors;
- global permissions for the concrete resource type;
- resource-specific entries in `ResourceAccesses`;
- the existing `IsAdmin` bypass.

Use the authorization semantics in
`src/Citadel.Infrastructure/Persistence/AuthorizationSql.cs`, including
`ActorScopeCte`, `GlobalAccessCte`, and the resource predicate. Do not perform a
per-result `IPermissionEvaluator` call.

The current SQL CTEs do not themselves express the `IsAdmin` bypass used by
`PermissionEvaluator`. The search handler/repository contract must therefore
carry trusted current-user admin state or choose a separate unrestricted query
path for admins. Never infer admin status from request input.

Do not put a single `[RequirePermission]` attribute for one resource type on
the multi-resource query. Such an attribute would incorrectly deny users who
can read another supported type and would not replace resource-level
filtering. The route requires authentication; the query performs authorization
for every requested concrete type.

## 13. Backend Design

Follow the existing minimal API, Mediator, Unit of Work, and Dapper structure.

Expected responsibilities:

- `PublicEndpoints.cs`: register a `/search` group below `/api/v1`, tag it, and
  require authorization.
- Web API endpoint: bind and validate query parameters, send one Mediator
  query, and map the result.
- Application query/handler: normalize requested types, obtain the trusted
  current user, coordinate searches, apply the global response cap, map status
  semantics, and group concrete results into the eight UI categories.
- Infrastructure persistence: perform permission filtering, matching, ranking,
  and per-type limits in PostgreSQL.

The existing `/api/v1/lookup` endpoint returns only `ResourceInfo(Id, Name)`
and is intentionally insufficient for global search. It may be used as a
reference for supported resources and authorization, but global search needs
ranked matches, status, secondary text, and independently authorized parents.
Do not change lookup behavior to satisfy this feature.

Prefer focused search projections and a small `IGlobalSearchRepository`
integrated with the existing Unit of Work over adding methods that return full
domain entities. Avoid:

- one query per result;
- loading all authorized rows into memory;
- a generic plugin/provider framework;
- dynamic table or column names derived from request data;
- a large cross-resource entity model.

Running one bounded query per requested concrete type is acceptable for the
MVP if queries are issued with controlled concurrency and one shared database
connection is not used concurrently. A `UNION ALL` query is also acceptable
when it remains readable and preserves per-type authorization parameters and
limits.

No database migration is expected. If implementation reveals that a required
search/status field is not persisted, update this specification before adding
schema solely for search.

## 14. JSON And Generated API Artifacts

Add request/response models and enum converters to:

```text
src/Citadel.WebApi/Routes/Endpoints/STJContext/ApplicationJsonContext.cs
```

After the route is implemented:

1. Build `src/Citadel.WebApi/Citadel.WebApi.csproj` to regenerate
   `src/schema/Citadel.WebApi.json`.
2. Synchronize that schema to
   `src/Citadel.FrontEnd/src/api/schema/swagger.json`.
3. Run `npm run api:generate` from `src/Citadel.FrontEnd`.
4. Commit the generated frontend API types/resources with the endpoint change.

Do not hand-edit generated API files.

## 15. Frontend Design

Suggested file ownership:

```text
src/Citadel.FrontEnd/src/features/search/
  global-search.tsx
  global-search-metadata.ts
  global-search-result.tsx
```

Exact file names may follow a nearby feature convention, but search-specific
UI must not be embedded in `header.tsx`.

Use:

- React Router's existing navigation APIs;
- the generated API operation;
- TanStack Query through the existing query hook pattern;
- `useDebounce`;
- `CitadelIcons`;
- the existing shadcn `CommandDialog` components.

Do not introduce TanStack Router, a second icon map, hard-coded backend URLs,
or result URLs supplied by the API.

## 16. Performance And Security

- Target a warm-database p95 below 300 ms for the default ten-type query.
- Do not issue a request before the minimum query length.
- Do not return more than 40 items.
- Parameterize all SQL values.
- Apply authorization in the same SQL operation that selects each resource.
- Do not log raw query values at information level.
- Do not cache results across users.
- If caching is added later, include the actor/effective authorization scope in
  the key and define invalidation for permission changes.
- Use `pg_trgm` GIN indexes for every searched field. Use partial indexes for
  resources where archived rows are excluded.
- Two-character substring searches may use sequential scans because PostgreSQL
  cannot extract a full trigram. Preserve substring semantics for those terms.
- Apply the per-type limit before parent enrichment and latest-run lookups.
- Latest-run indexes must cover the parent ID, descending queue timestamp,
  descending run ID, and include the returned status.

## 17. Tests

### 17.1 Backend Integration Tests

Add focused tests under the existing integration test project for:

- unauthenticated requests;
- query length and maximum length validation;
- invalid concrete resource types and limits;
- case-insensitive exact, prefix, substring, and secondary-field matching;
- deterministic ranking and limits;
- global read permission;
- direct resource-level read access;
- enabled-team inherited access;
- disabled actor/team exclusion;
- admin access;
- mixed access across multiple resource types;
- omission of an inaccessible parent;
- inclusion of an independently accessible parent;
- Backups grouping both backup concrete types;
- Builds grouping both build concrete types;
- no secret or sensitive fields in serialized output;
- zero results returning `200` with empty groups.

Use PostgreSQL-backed integration tests for matching and authorization SQL.
Unit tests alone are not sufficient for `ILIKE`, escaping, CTE, and bit-mask
behavior.

### 17.2 Frontend Tests

Cover:

- header click and `Ctrl/Cmd+K` opening;
- short queries not requesting the API;
- debounce and stale-request behavior;
- category navigation in the empty state;
- group ordering;
- result and parent navigation as separate actions;
- keyboard selection, Escape, and focus restoration;
- loading, no-results, error, and retry states;
- narrow header rendering;
- status text remaining visible independently of color.

### 17.3 Manual Verification

Verify with users that have:

- admin access;
- one globally readable resource type;
- only resource-specific access;
- access through an enabled team;
- child access without parent access;
- no access to any searchable resource.

Confirm that restricted resource names, IDs, parents, totals, and statuses are
not observable in the response or UI.

## 18. Acceptance Criteria

The MVP is complete when:

1. An authenticated user can open global search from the header or keyboard.
2. Empty search displays the eight supported categories.
3. Two or more characters produce grouped, ranked results.
4. Backups and Builds correctly combine their two concrete resource types.
5. Result navigation uses current Citadel routes.
6. Result and parent links work independently.
7. Status text and semantic indicators match current resource state.
8. The API returns only resources with effective read access.
9. Parent metadata is returned only with independent parent read access.
10. Admin, direct, global, and enabled-team access behave like existing lists.
11. Search is bounded, parameterized, and performs filtering in PostgreSQL.
12. OpenAPI and generated frontend API artifacts are updated.
13. Backend and frontend checks pass.

## 19. Exclusions

The MVP does not include:

- full-text indexing or fuzzy search;
- external search infrastructure;
- searching logs, code, configuration bodies, or secret-bearing values;
- command execution or create actions;
- recent searches, saved searches, or personalization;
- resource counts in the empty-query category list;
- advanced filters beyond the optional concrete resource-type filter;
- server-provided routes or icons;
- pagination or infinite scrolling;
- searching resources outside the ten concrete types in section 3;
- schema changes solely for search optimization without measured evidence.

## 20. Implementation Order

1. Add the API contracts and source-generated JSON metadata.
2. Implement permission-scoped persistence queries and the Application handler.
3. Register the authenticated endpoint and add backend integration tests.
4. Regenerate and synchronize OpenAPI and frontend API artifacts.
5. Add the centralized frontend metadata.
6. Implement the header trigger, popup states, results, and navigation.
7. Add frontend interaction and accessibility tests.
8. Run backend and frontend checks and fix regressions.

Do not rename existing domain resources or routes to match this document.
