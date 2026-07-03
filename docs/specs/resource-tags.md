# Resource Tags

## Purpose

Citadel needs a global tag catalog so users can organize and filter core resources without encoding environment, customer, or ownership hints into resource names.

Initial taggable resources:

- Deployments
- Stacks
- Platforms
- Git repositories

Tags are global catalog entries. Admins manage the catalog. Users can assign existing tags to a resource when they have normal write permission on that resource.

Examples:

- `Dev`
- `Prod`
- `Staging`
- `Customer-A`
- `Experimental`

## Main Files

Add new tag code beside existing feature, endpoint, domain, repository, and frontend patterns.

- `src/Citadel.Domain/Entities/Tags`
- `src/Citadel.Domain/Enums.cs`
- `src/Citadel.Domain/Contracts/Interfaces/IUnitOfWork.cs`
- `src/Citadel.Application/Features.Tags`
- `src/Citadel.Infrastructure/Persistence`
- `src/Citadel.Infrastructure.Migrations/Migrations`
- `src/Citadel.WebApi/Routes/Endpoints/Tags.cs`
- `src/Citadel.WebApi/Routes/Endpoints/Resources/Tags`
- `src/Citadel.WebApi/Routes/PublicEndpoints.cs`
- `src/Citadel.FrontEnd/src/features/tags`
- `src/Citadel.FrontEnd/src/components/custom`
- `src/Citadel.FrontEnd/src/features/deployments`
- `src/Citadel.FrontEnd/src/features/stacks`
- `src/Citadel.FrontEnd/src/features/platforms`
- `src/Citadel.FrontEnd/src/features/git-repos`

## MVP Scope

In scope:

- Create global tags.
- Rename global tags.
- Change tag colors.
- Delete global tags.
- Assign multiple tags when creating supported resources.
- Replace assigned tags from supported resource edit/detail flows.
- Display tag chips on list rows.
- Display tag chips on detail pages.
- Filter supported list pages by one or more tags.
- Add a global admin Tags page.

Out of scope:

- Per-user tags.
- Nested tags.
- Tag descriptions.
- Tag categories.
- Tag permissions.
- Tag-based RBAC.
- Auto-tagging.
- Tag merge workflows.
- Tag history pages.
- Usage analytics beyond a simple usage count.

## Product Rules

Tags are global, not resource-scoped definitions.

Only admins can create, update, or delete tags. Non-admin users can read the tag list so they can assign existing tags.

Users can assign or remove tags from a resource when they have `PermissionLevel.Write` on that resource. Do not add tag-specific assignment permissions for the MVP.

Filtering applies on list pages only:

- Deployments list
- Stacks list
- Platforms list
- Git repositories list

Filtering uses `OR` matching:

```text
Selected tags: Prod, Backend
Result: resources with Prod OR Backend
```

Do not implement `AND` matching in the first slice.

## Naming

Use "Tags" in UI and docs.

Use `Tag` and `ResourceTag` in domain code.

Use a dedicated taggable resource enum instead of accepting arbitrary strings from clients:

```csharp
public enum TaggableResourceType
{
    Deployment,
    Stack,
    Platform,
    GitRepository
}
```

Persist enum values as strings. Keep values stable because they become part of the generic join table key.

## Data Model

Citadel's generated database schema uses lowercase table and column names. Use the same convention in migrations and SQL.

### Table: `tags`

```sql
CREATE TABLE tags (
    id uuid NOT NULL,
    name text NOT NULL,
    normalizedname text NOT NULL,
    color text NOT NULL,
    createdbyactorid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updatedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    CONSTRAINT pk_tags PRIMARY KEY (id),
    CONSTRAINT fk_tags_actors_createdbyactorid
        FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX ix_tags_normalizedname ON tags (normalizedname);
CREATE INDEX ix_tags_createdbyactorid ON tags (createdbyactorid);
```

### Table: `resourcetags`

Use one generic join table for every supported resource type.

```sql
CREATE TABLE resourcetags (
    resourcetype text NOT NULL,
    resourceid uuid NOT NULL,
    tagid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    CONSTRAINT pk_resourcetags PRIMARY KEY (resourcetype, resourceid, tagid),
    CONSTRAINT fk_resourcetags_tags_tagid
        FOREIGN KEY (tagid) REFERENCES tags (id) ON DELETE CASCADE,
    CONSTRAINT fk_resourcetags_actors_createdbyactorid
        FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE INDEX ix_resourcetags_tagid ON resourcetags (tagid);
CREATE INDEX ix_resourcetags_resource ON resourcetags (resourcetype, resourceid);
CREATE INDEX ix_resourcetags_filter ON resourcetags (resourcetype, tagid, resourceid);
```

Use `ON DELETE CASCADE` from `resourcetags.tagid` to `tags.id`. Explicit delete ordering is still acceptable in the repository, but the FK should protect direct database operations.

Do not add foreign keys from `resourcetags.resourceid` to each supported resource table. The generic table cannot enforce those constraints cleanly without separate join tables.

## Field Rules

### `Name`

- Required.
- Trim whitespace.
- Max length: `64`.
- Must not be empty after trimming.
- Must be unique case-insensitively.
- Preserve casing for display.

These names conflict:

```text
Prod
prod
PROD
```

They all normalize to:

```text
prod
```

### `NormalizedName`

Generated server-side only:

```text
trim -> lowercase invariant
```

Do not accept `normalizedName` from API inputs.

### `Color`

- Required.
- Must be a hex color in `#RRGGBB` format.
- Normalize to uppercase before persistence.

Valid:

```text
#22C55E
#EF4444
#3B82F6
```

Invalid:

```text
red
#FFF
22C55E
```

## Domain Types

Add `Domain.Entities.Tags.Tag`:

```csharp
public sealed class Tag
{
    public Guid Id { get; init; }
    public string Name { get; private set; } = default!;
    public string NormalizedName { get; private set; } = default!;
    public string Color { get; private set; } = default!;
    public Guid CreatedByActorId { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
    public DateTimeOffset UpdatedAt { get; private set; }

    public static Tag Create(string name, string color, Guid createdByActorId, DateTimeOffset now);
    public void RenameAndRecolor(string name, string color, DateTimeOffset now);
}
```

Add `Domain.Entities.Tags.ResourceTag`:

```csharp
public sealed class ResourceTag
{
    public TaggableResourceType ResourceType { get; init; }
    public Guid ResourceId { get; init; }
    public Guid TagId { get; init; }
    public DateTimeOffset CreatedAt { get; init; }
    public Guid CreatedByActorId { get; init; }
}
```

Add tag validation in the domain or application layer:

```csharp
public static class TagValidation
{
    public const int MaxNameLength = 64;

    public static string NormalizeName(string name)
        => name.Trim().ToLowerInvariant();

    public static string NormalizeColor(string color)
        => color.Trim().ToUpperInvariant();

    public static bool IsValidColor(string color)
        => Regex.IsMatch(color, "^#[0-9A-Fa-f]{6}$");
}
```

Prefer a generated regex if this helper lives in a non-AOT-hostile location.

## DTOs And Views

Follow the current minimal API endpoint style: command inputs and HTTP views live under `src/Citadel.WebApi/Routes/Endpoints/Resources/...`, while application commands return domain/results.

Add:

```csharp
public sealed record TagView(
    Guid Id,
    string Name,
    string NormalizedName,
    string Color,
    Guid CreatedByActorId,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt,
    int UsageCount);

public sealed record TagSummaryView(
    Guid Id,
    string Name,
    string Color);

public sealed record CreateTagInput(string Name, string Color);

public sealed record PatchTagInput(string? Name, string? Color);

public sealed record ReplaceResourceTagsInput(IReadOnlyList<Guid>? TagIds);
```

Use `TagSummaryView` in every resource list/detail response. Keep it lightweight and do not expose `normalizedName` on resource rows.

## Repository Interfaces

Extend `IUnitOfWork`:

```csharp
ITagRepository Tags { get; }
IResourceTagRepository ResourceTags { get; }
```

Add:

```csharp
public interface ITagRepository
{
    Task<IReadOnlyList<TagWithUsage>> ListAsync(CancellationToken cancellationToken);
    Task<Tag?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Tag?> GetByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken);
    Task<int> AddAsync(Tag tag, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Tag tag, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
}
```

Add:

```csharp
public interface IResourceTagRepository
{
    Task<IReadOnlyList<TagSummary>> GetForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        CancellationToken cancellationToken);

    Task<IReadOnlyDictionary<Guid, IReadOnlyList<TagSummary>>> GetForResourcesAsync(
        TaggableResourceType resourceType,
        IReadOnlyCollection<Guid> resourceIds,
        CancellationToken cancellationToken);

    Task<int> ReplaceForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        IReadOnlyCollection<Guid> tagIds,
        Guid createdByActorId,
        DateTimeOffset now,
        CancellationToken cancellationToken);

    Task<bool> AllTagsExistAsync(
        IReadOnlyCollection<Guid> tagIds,
        CancellationToken cancellationToken);
}
```

`GetForResourcesAsync` is required. Do not load tags one resource at a time on list pages.

## Application Features

Create `src/Citadel.Application/Features.Tags`.

Suggested commands and queries:

- `GetTags`
- `CreateTag`
- `PatchTag`
- `DeleteTag`
- `GetResourceTags`
- `ReplaceResourceTags`

Global tag mutation commands must require admin-only semantics. The cleanest long-term model is adding `ResourceType.Tag` to `Hosting.Common.ResourceType` and `PermissionMatrix`, then applying:

```csharp
[RequirePermission(ResourceType.Tag, PermissionLevel.Write)]
```

If the implementation deliberately avoids a permission matrix expansion in the first pass, use `ResourceType.Binding` for tag catalog mutation because tags are configuration metadata. Document that as temporary technical debt.

Tag reads should require authentication but not write/admin rights.

Resource assignment commands must require write permission on the target resource:

```csharp
[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
[RequirePermission(ResourceType.GitRepository, PermissionLevel.Write)]
```

Because the target resource type is dynamic, implement dispatch commands per resource type or have one dynamic command that performs an explicit permission check through the existing permission service. Avoid trusting client-provided resource type strings.

## API

All endpoints live under `/api/v1`.

Add a new group in `PublicEndpoints.cs`:

```csharp
var tags = group.MapGroup("/tags").WithTags("Tags").RequireAuthorization();
```

### `GET /api/v1/tags`

Returns all tags with usage counts.

Response:

```json
[
  {
    "id": "019f0000-0000-7000-8000-000000000001",
    "name": "Prod",
    "normalizedName": "prod",
    "color": "#EF4444",
    "createdByActorId": "00000000-0000-0000-0000-000000000002",
    "createdAt": "2026-07-03T12:00:00Z",
    "updatedAt": "2026-07-03T12:00:00Z",
    "usageCount": 12
  }
]
```

### `POST /api/v1/tags`

Admin-only.

Request:

```json
{
  "name": "Prod",
  "color": "#EF4444"
}
```

Behavior:

- Trim and validate name.
- Validate and normalize color.
- Normalize name.
- Reject duplicate normalized names.
- Create tag.

Return `200 OK` or `201 Created` consistently with nearby Citadel endpoints. Existing create endpoints generally return `Ok<T>`, so `200 OK` is acceptable.

### `PATCH /api/v1/tags/{tagId}`

Admin-only.

Use the existing merge-patch pattern if partial patch documents are desired:

```json
{
  "name": "Production",
  "color": "#DC2626"
}
```

Behavior:

- Validate tag exists.
- Apply provided fields.
- Validate resulting name and color.
- Reject duplicate normalized names from another tag.
- Update `Name`, `NormalizedName`, `Color`, and `UpdatedAt`.

### `DELETE /api/v1/tags/{tagId}`

Admin-only.

Behavior:

- Delete resource bindings through cascade or explicit repository delete.
- Delete tag.
- Return `204 No Content`.

Deleting a tag removes it from all resources. Do not block deletion because a tag is used.

### Resource Tag Endpoints

Use dedicated resource endpoints for edit flows and optional `tagIds` on create inputs for create flows.

Create inputs:

- Add `IReadOnlyList<Guid>? TagIds` to `CreateDeploymentInput`.
- Add `IReadOnlyList<Guid>? TagIds` to `CreateStackInput`.
- Add `IReadOnlyList<Guid>? TagIds` to `PlatformInput` or the platform create input path.
- Add `IReadOnlyList<Guid>? TagIds` to `CreateGitRepositoryInput`.

Resource edit endpoints:

```http
GET /api/v1/deployments/{deploymentId}/tags
PUT /api/v1/deployments/{deploymentId}/tags

GET /api/v1/stacks/{stackId}/tags
PUT /api/v1/stacks/{stackId}/tags

GET /api/v1/platforms/{platformId}/tags
PUT /api/v1/platforms/{platformId}/tags

GET /api/v1/gitRepositories/{gitRepositoryId}/tags
PUT /api/v1/gitRepositories/{gitRepositoryId}/tags
```

Request:

```json
{
  "tagIds": [
    "019f0000-0000-7000-8000-000000000001",
    "019f0000-0000-7000-8000-000000000002"
  ]
}
```

Rules:

- `tagIds` is optional during create.
- Missing `tagIds` means no tags.
- Empty `tagIds` means remove all tags.
- Deduplicate IDs server-side.
- Reject unknown tag IDs with `400 Bad Request`.
- Replace the full current tag set on update.
- Use the authenticated actor as `CreatedByActorId` for newly inserted bindings.
- Resource create and tag insert must commit in one unit of work.
- Resource tag replacement and any surrounding resource update must commit in one unit of work when they are part of the same user action.

## Resource List And Detail Responses

Add `tags` to supported list/detail views:

```json
{
  "id": "019f0000-0000-7000-8000-000000000010",
  "name": "my-api",
  "status": "Healthy",
  "tags": [
    {
      "id": "019f0000-0000-7000-8000-000000000001",
      "name": "Prod",
      "color": "#EF4444"
    }
  ]
}
```

Affected mapper files include:

- `Resources/Deployments/DeploymentsView.cs`
- `Resources/Stacks/StacksView.cs`
- `Resources/Platforms/PlatformsView.cs`
- `Resources/GitRepositories/GitRepositoriesView.cs`

Do not perform tag queries inside individual view mapping loops. Query tags in the application/repository layer or batch before mapping.

## Filtering

Add a tag filter to list endpoints.

Preferred query style for ASP.NET minimal API binding:

```http
GET /api/v1/deployments?tagIds=019f...&tagIds=019f...
GET /api/v1/stacks?tagIds=019f...&tagIds=019f...
GET /api/v1/platforms?tagIds=019f...
GET /api/v1/gitRepositories?tagIds=019f...
```

If the frontend strongly prefers comma-separated values, add a small parser and document it. Keep frontend and backend consistent.

SQL concept:

```sql
WHERE EXISTS (
    SELECT 1
    FROM resourcetags rt
    WHERE rt.resourcetype = @ResourceType
      AND rt.resourceid = resource.id
      AND rt.tagid = ANY(@TagIds)
)
```

No `tagIds` means no tag filter.

Filter in the repository query when possible so the API does not fetch large authorized lists and then discard rows in memory.

## Error Handling

Use existing `LightResults` errors and endpoint problem handling.

Duplicate tag name:

```json
{
  "code": "tag_name_already_exists",
  "message": "A tag with this name already exists."
}
```

Invalid color:

```json
{
  "code": "invalid_tag_color",
  "message": "Tag color must be a valid hex color in #RRGGBB format."
}
```

Unknown tag IDs:

```json
{
  "code": "invalid_tag_ids",
  "message": "One or more tag IDs do not exist."
}
```

Non-admin tag mutation:

```text
403 Forbidden
```

## Activity Events

Keep activity noise low.

Do not create one activity event per tag binding.

When tags are changed from a resource edit flow, create one summarized resource activity event if the surrounding resource already records one. Prefer extending existing updated events before adding new event types.

Example message:

```text
Tags updated for stack "immich": added Prod, Backend; removed Experimental.
```

If structured metadata is used, store tag IDs and names:

```json
{
  "addedTagIds": ["019f..."],
  "removedTagIds": ["019f..."],
  "addedTagNames": ["Prod"],
  "removedTagNames": ["Experimental"]
}
```

Do not include tag activity for automatic cascade deletion unless a user explicitly deleted the tag.

## Frontend

Create `src/Citadel.FrontEnd/src/features/tags`.

Global route:

```text
/tags
```

If the navigation later gains a settings/admin area, `/settings/tags` is also acceptable. Keep the first implementation aligned with existing top-level feature routes such as `bindings`.

### Global Tags Page

Admin-only page. Hide the route/nav item for non-admin users, but rely on backend write permissions for enforcement.

Minimum table columns:

- Name
- Color
- Usage count
- Created by
- Created at
- Updated at
- Actions

Actions:

- Create
- Edit
- Delete

### Create/Edit Dialog

Fields:

- Name input.
- Color preset selector.
- Hex color text input.
- Preview chip.

Validation:

- Name required.
- Name max length 64.
- Color required.
- Color must match `#RRGGBB`.

Use a preset palette for speed, but keep text input available for exact colors.

### Reusable Components

Add shared components under `src/Citadel.FrontEnd/src/components/custom` when useful:

- `tag-chip.tsx`
- `tag-chip-list.tsx`
- `tag-multi-select.tsx`
- `tag-filter.tsx`

`TagMultiSelect` behavior:

- Loads global tags from `GET /api/v1/tags`.
- Supports multiple selection.
- Displays selected tags as chips.
- Allows searching by name.
- Does not create tags inline in MVP.

Empty states:

- Admin: `No tags yet. Create tags from the Tags page.`
- Non-admin: `No tags available.`

### List Rows

Add a Tags column to supported resource tables.

Rules:

- Show up to 3 tags inline.
- If there are more than 3, show `+N`.
- Show all tags in a tooltip or popover.
- Use the tag color as a small swatch or chip accent.
- Do not let tags resize table rows unpredictably.

### Detail Pages

Show tags near the resource title or metadata area.

Users with write permission can edit assigned tags from the normal edit/detail workflow. Non-writers see read-only chips.

### List Filters

Add a tag filter near existing search/filter controls.

Rules:

- Multi-select.
- Selection updates URL query params.
- Query params are bookmarkable.
- Clearing the filter removes all `tagIds` params.

Example frontend URL:

```text
/stacks?tagIds=019f0000-0000-7000-8000-000000000001&tagIds=019f0000-0000-7000-8000-000000000002
```

## Migration Requirements

Add an EF migration for:

- `tags`
- `resourcetags`
- indexes
- FK to `actors`
- FK from `resourcetags.tagid` to `tags.id`

Update:

- `src/Citadel.Infrastructure.Migrations/Migrations/ApplicationDbContextModelSnapshot.cs`
- generated SQL/script if this repo keeps script snapshots current

Seed tags only if demo data is desired. If seeded, use the system actor:

```text
00000000-0000-0000-0000-000000000001
```

Suggested seed values:

- `Prod` `#EF4444`
- `Staging` `#F59E0B`
- `Dev` `#3B82F6`

Do not attach seeded tags to resources.

## Backend Tests

Add focused tests under `test/Citadel.Tests.Unit` and integration/API tests where existing coverage for endpoints lives.

Tag creation:

- Admin can create tag.
- Non-admin cannot create tag.
- Name is trimmed.
- Duplicate name with different casing is rejected.
- Invalid color is rejected.
- Color is normalized.

Tag update:

- Admin can rename tag.
- Duplicate normalized name is rejected.
- Color can be changed.
- Invalid color is rejected.

Tag deletion:

- Admin can delete tag.
- Deleting a tag removes resource bindings.
- Non-admin cannot delete tag.

Resource assignment:

- User with create permission can assign tags during create.
- User with update permission can replace tags.
- Unknown tag IDs are rejected.
- Duplicate tag IDs are deduplicated.
- Missing `tagIds` does not break existing create flows.
- Empty `tagIds` removes all tags.

Filtering:

- List returns all authorized resources when no tag filter is provided.
- List returns resources matching one tag.
- List uses OR behavior for multiple tags.
- Unauthorized resources are still excluded when tag filters are present.
- List/detail responses include assigned tag DTOs.
- List tag loading avoids N+1 behavior.

## Frontend Tests

Recommended:

- Tags page create/edit/delete interactions.
- Tag form validation.
- Read-only behavior for non-admin tag catalog.
- Tag selector search and selection.
- Tag chips in resource lists.
- Tag filter updates URL params.
- Resource list query includes selected tag IDs.
- Resource edit page preserves existing assigned tags.

## Implementation Slices

### Slice 1: Backend Catalog

- Add domain types and validation helper.
- Add migration.
- Add `ITagRepository`.
- Add `IResourceTagRepository`.
- Add `Features.Tags` catalog commands/queries.
- Add `/api/v1/tags` endpoints.
- Add backend tests for catalog validation and permissions.

### Slice 2: Resource Assignment

- Add optional `tagIds` to supported create inputs/commands.
- Add resource tag replacement endpoints.
- Wire create handlers transactionally.
- Add assignment tests.

### Slice 3: Resource Read Models

- Add tags to list/detail responses.
- Add batched tag loading.
- Add repository-level OR filtering by `tagIds`.
- Add filtering tests.

### Slice 4: Frontend Catalog

- Add tag API client/hooks.
- Add global Tags page.
- Add create/edit/delete dialog.
- Add tag chips and multi-select components.

### Slice 5: Frontend Resource Integration

- Add tag selector to create/edit flows.
- Add tag chips to lists/details.
- Add tag filters to list pages.
- Add frontend tests.

## Acceptance Criteria

The feature is complete when:

- Admins can create, edit, and delete global tags.
- Non-admin users cannot mutate global tags.
- Authenticated users can read existing tags.
- Users can assign existing tags to resources they are allowed to create/update.
- Deployments, stacks, platforms, and git repositories can have multiple tags.
- Resource list responses include assigned tags.
- Resource detail responses include assigned tags.
- Resource list pages can be filtered by one or more tags.
- Filtering uses OR behavior.
- Tags are displayed as colored chips in the UI.
- Duplicate tag names are rejected case-insensitively.
- Invalid colors are rejected.
- Backend list queries avoid N+1 tag loading.
- Existing resource create flows continue working when `tagIds` is omitted.

## Open Decisions

- Add `ResourceType.Tag` now, or temporarily reuse `ResourceType.Binding` for tag catalog permissions.
- Use `/tags` or `/settings/tags` for the frontend route.
- Accept only repeated `tagIds` query params, or also accept comma-separated values.
- Whether resource tag changes should create new activity event types or extend existing resource updated events.
