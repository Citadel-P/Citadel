# Duplicate Deployment and Stack

## Goal

Allow a user to create a new Deployment or Stack from the editable configuration of an existing resource.

Duplication is a draft-only flow. Selecting `Duplicate` must not create anything. The user lands on the existing add form, reviews or changes the generated draft, and submits through the normal create endpoint.

## Citadel Alignment

Use existing Citadel routes and resource patterns:

- Frontend add routes are `/deployments/add` and `/stacks/add`, not `/new`.
- Public API routes are under `/api/v1`.
- Existing create endpoints remain the only endpoints that create resources.
- Duplicate draft loading is a query, not a command.
- Activity events are resource-specific, not generic.
- Generated frontend API files under `src/Citadel.FrontEnd/src/api/generated` must be regenerated, not edited manually.

## User Flow

Add a `Duplicate` action to:

- Deployment action menu.
- Stack action menu.

When the action is selected:

1. Navigate to `/deployments/add?duplicateFrom={deploymentId}` or `/stacks/add?duplicateFrom={stackId}`.
2. The add form requests a sanitized draft from the backend.
3. The add form is populated with the draft.
4. The form displays any draft warnings.
5. The user edits the draft normally.
6. The user submits through the existing create endpoint.

Default duplicate name:

```text
<source-name>-copy
```

If that name conflicts, the draft endpoint returns the first available numeric suffix:

```text
<source-name>-copy-2
<source-name>-copy-3
```

Generated names must stay within Citadel's normal resource-name rules, including the 64-character limit.

## API Contract

Add draft endpoints:

```http
GET /api/v1/deployments/{deploymentId}/duplicate-draft
GET /api/v1/stacks/{stackId}/duplicate-draft
```

Responses:

```csharp
public sealed record DuplicateDraftWarning(
    string Code,
    string Message,
    string? FieldPath = null);

public sealed record DeploymentDuplicateDraftView(
    CreateDeploymentInput Draft,
    IReadOnlyCollection<DuplicateDraftWarning> Warnings);

public sealed record StackDuplicateDraftView(
    CreateStackInput Draft,
    IReadOnlyCollection<DuplicateDraftWarning> Warnings);
```

Example JSON:

```json
{
  "draft": {
    "name": "api-copy",
    "platformId": "019f0000-0000-7000-8000-000000000001",
    "description": "Production API",
    "spec": {},
    "tagIds": []
  },
  "warnings": [
    {
      "code": "HOST_BIND_MOUNT",
      "message": "This deployment contains host paths that may not exist on another platform.",
      "fieldPath": "spec.volumes"
    }
  ]
}
```

Do not use the existing `/{id}/_cfg` endpoints as the duplicate API. They are edit-config endpoints and currently do not include everything needed for a create draft, such as tag IDs and duplicate warnings.

## Create Contract

Keep the existing create endpoints:

```http
POST /api/v1/deployments
POST /api/v1/stacks
```

Extend the normal create inputs with optional duplicate provenance:

```csharp
public sealed record DuplicateSourceInput(
    ActivityResourceType ResourceType,
    Guid ResourceId,
    string ResourceName);
```

Deployment:

```csharp
public sealed record CreateDeploymentInput(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec,
    IReadOnlyCollection<Guid>? TagIds = null,
    DuplicateSourceInput? DuplicateSource = null)
```

Stack:

```csharp
public sealed record CreateStackInput(
    string Name,
    Guid PlatformId,
    string? Description,
    StackSource StackSource,
    StackSpec Spec,
    StackDriftPolicy? DriftPolicy = null,
    IReadOnlyCollection<Guid>? TagIds = null,
    DuplicateSourceInput? DuplicateSource = null)
```

The final create handlers must not trust `DuplicateSource`. If it is supplied, re-check that the source resource still exists and that the actor can read it before writing a duplicate activity event.

If provenance validation fails, return an authorization or not-found failure instead of writing a forged activity event.

## Backend Implementation

Add query handlers:

- `src/Citadel.Application/Features.Deployments/Queries/GetDeploymentDuplicateDraft.cs`
- `src/Citadel.Application/Features.Stacks/Queries/GetStackDuplicateDraft.cs`

Add endpoint methods:

- `Deployments.GetDuplicateDraft`
- `Stacks.GetDuplicateDraft`

Register routes in `src/Citadel.WebApi/Routes/PublicEndpoints.cs` near the existing `/{id}/_cfg` routes:

```csharp
deployments.MapGet("{deploymentId}/duplicate-draft", Deployments.GetDuplicateDraft)
    .WithSummary("Get deployment duplicate draft")
    .WithName("getDeploymentDuplicateDraft");

stacks.MapGet("{stackId}/duplicate-draft", Stacks.GetDuplicateDraft)
    .WithSummary("Get stack duplicate draft")
    .WithName("getStackDuplicateDraft");
```

Add response/input records under the existing resource endpoint folders:

- `src/Citadel.WebApi/Routes/Endpoints/Resources/Deployments`
- `src/Citadel.WebApi/Routes/Endpoints/Resources/Stacks`

Add new serializable types to `ApplicationJsonContext` as needed.

Regenerate the OpenAPI client after backend changes. Do not manually edit `src/Citadel.FrontEnd/src/api/generated/resources.ts`.

## Draft Construction

Deployment draft:

```csharp
new CreateDeploymentInput(
    Name: await GetAvailableDuplicateNameAsync(source.Name, source.PlatformId, cancellationToken),
    PlatformId: source.PlatformId,
    Description: source.Description,
    Spec: sanitizedSpec,
    TagIds: source.Tags.Select(x => x.Id).ToArray())
```

Stack draft:

```csharp
new CreateStackInput(
    Name: await GetAvailableDuplicateNameAsync(source.Name, cancellationToken),
    PlatformId: source.CurrentStackRelease.PlatformId,
    Description: source.Description,
    StackSource: source.StackSource,
    Spec: sanitizedSpec,
    DriftPolicy: source.DriftPolicy,
    TagIds: source.Tags.Select(x => x.Id).ToArray())
```

If a Stack has no current release/spec to duplicate, return a failure instead of producing an invalid draft.

## Deployment Copy Rules

Copy editable fields:

- Description.
- Platform ID.
- Tag IDs.
- Deployment spec.
- Image configuration.
- Registry reference.
- Environment variables.
- Port mappings.
- Volumes.
- Networks.
- Labels.
- Resource limits.
- Restart policy.
- Command and entrypoint.
- Update behavior.
- Other editable fields already carried by `DeploymentSpec`.

Do not copy:

- Deployment ID.
- Original name unchanged.
- Runtime status.
- Control state.
- Container IDs.
- Health state.
- Activity history.
- Creation metadata.
- Row version or concurrency metadata.
- Resolved secret values.
- Generated credentials.

Resource bindings are not duplicated in the MVP. The draft may preserve textual variable or secret references in the spec, but it must not create new `ResourceBinding` rows or expose resolved values.

## Stack Copy Rules

Copy editable fields:

- Description.
- Platform ID from the current stack release.
- Tag IDs.
- Stack source.
- Manual Compose configuration.
- Git repository reference.
- Branch.
- Compose paths.
- Project name.
- Environment variables.
- Registry reference.
- Drift policy.
- Update behavior.
- Destroy-before-deploy setting.
- Pre-deploy and post-deploy configuration.
- Other editable fields already carried by `StackSpec`.

Do not copy:

- Stack ID.
- Original name unchanged.
- Current stack release ID.
- Release version.
- Release status.
- Release history.
- Apply history.
- Runtime container state.
- Control state.
- Activity history.
- Webhook secrets.
- Generated webhook credentials.
- Creation metadata.
- Row version or concurrency metadata.
- Resolved secret values.

For Git-backed stacks, copy the configured repository, branch, and Compose paths.

Do not copy a commit SHA that was only resolved during the latest pull. Preserve the commit SHA only when the editable stack configuration explicitly pins the stack to that commit.

Resource bindings are not duplicated in the MVP. The draft may preserve textual variable or secret references in Compose content, but it must not create new `ResourceBinding` rows or expose resolved values.

## Warnings

The draft endpoint may return warnings. Warnings do not block the form.

Initial warning codes:

- `HOST_BIND_MOUNT`: host paths may not exist on another platform.
- `EXTERNAL_NETWORK`: external Docker networks may not exist on another platform.
- `RESOURCE_BINDINGS_NOT_COPIED`: source resource has resource-scoped bindings that must be recreated manually.
- `WEBHOOK_SECRET_NOT_COPIED`: stack webhook generated credentials were intentionally omitted.
- `PLATFORM_SPECIFIC_CONFIG`: config contains values likely tied to the source platform.

## Permissions

Draft query:

- The actor must be able to read the source resource.
- The actor should have write/create capability for the same resource type so the frontend does not offer a dead-end flow.

Final create:

- Reuse existing create command permissions:
  - `CreateDeployment` requires `ResourceType.Deployment` write permission.
  - `CreateStack` requires `ResourceType.Stack` write permission.
- Existing create validation still checks destination platform, tags, registry, git repository, variables, secrets, and other referenced resources.
- If `DuplicateSource` is supplied, validate source access again before writing duplicate activity.

## Activity

Do not add a generic `ResourceDuplicated` event. Citadel activity events are resource-specific.

Add:

- `ActivityEventType.DeploymentDuplicated`
- `ActivityEventType.StackDuplicated`

Add activity info records:

```csharp
public sealed record ActivitySourceResource(
    ActivityResourceType ResourceType,
    Guid ResourceId,
    string ResourceName);

public sealed record DeploymentDuplicated(
    DeploymentSnapshot Deployment,
    ActivitySourceResource Source) : ActivityEventInfo;

public sealed record StackDuplicated(
    StackSnapshot Stack,
    ActivitySourceResource Source) : ActivityEventInfo;
```

When a create request contains valid duplicate provenance, write the duplicated event on the newly created resource. Do not write a duplicate event on the source resource.

Use only one creation-time event for duplicate creation. Prefer `DeploymentDuplicated` or `StackDuplicated` instead of also writing `DeploymentCreated` or `StackCreated`, to avoid noisy duplicate activity.

Update:

- `src/Citadel.Domain/Enums.cs`
- `src/Citadel.Domain/Entities/Activities/ActivityEvent.cs`
- `src/Citadel.Domain/Entities/Activities/ActivityEventInfo.cs`
- `src/Citadel.WebApi/Hubs/SignalRSerializeContext.cs`
- `src/Citadel.WebApi/Routes/Endpoints/STJContext/ApplicationJsonContext.cs`
- Frontend activity labels/rendering in `src/Citadel.FrontEnd/src/features/activities`
- Task sheet rendering in `src/Citadel.FrontEnd/src/components/custom/task-sheet.tsx`

## Frontend Implementation

Add `Duplicate` action with a copy icon:

- `src/Citadel.FrontEnd/src/features/deployments/actions.tsx`
- `src/Citadel.FrontEnd/src/features/stacks/actions.tsx`

Action navigation:

```ts
navigate(`/deployments/add?duplicateFrom=${selected.id}`);
navigate(`/stacks/add?duplicateFrom=${selected.id}`);
```

Update add forms:

- `src/Citadel.FrontEnd/src/features/deployments/form/form.tsx`
- `src/Citadel.FrontEnd/src/features/stacks/form/form.tsx`

When `mode === "add"` and `duplicateFrom` is present:

1. Fetch the duplicate draft using the generated client.
2. Seed the form `update` state from `response.draft`.
3. Display warnings near the top of the form.
4. Keep every field editable.
5. Include `duplicateSource` when submitting the create request.

Use a duplicate-specific draft key so an unsaved normal add draft does not overwrite or get overwritten by a duplicate draft:

```tsx
draftKey={
  duplicateFrom
    ? `deployment:duplicate:${duplicateFrom}`
    : `deployment:${id ?? "new"}`
}
```

```tsx
draftKey={
  duplicateFrom
    ? `stack:duplicate:${duplicateFrom}`
    : `stack:${id ?? "new"}`
}
```

The form must not show edit-only tabs or controls for a draft that is not yet persisted.

## Tests

Application/query tests:

- Deployment duplicate draft copies editable configuration.
- Stack duplicate draft copies editable configuration from the current release.
- Tag IDs are copied.
- Default name is `<source-name>-copy`, with numeric suffixes when needed.
- Runtime state is excluded.
- Activity history is excluded.
- Stack release history is excluded.
- Webhook generated credentials are excluded.
- Resolved secret values are excluded.
- Git runtime-resolved commit SHA is excluded.
- Explicitly pinned commit SHA is preserved.
- Source without current stack release fails cleanly.
- Source without read permission is rejected.

Create handler tests:

- Normal create still records `DeploymentCreated` or `StackCreated`.
- Create with valid duplicate provenance records `DeploymentDuplicated` or `StackDuplicated`.
- Create with invalid duplicate provenance is rejected.
- Duplicate provenance is recorded on the new resource, not on the source resource.

Endpoint tests:

- `GET /api/v1/deployments/{id}/duplicate-draft` returns the expected draft shape.
- `GET /api/v1/stacks/{id}/duplicate-draft` returns the expected draft shape.
- Unauthorized and forbidden requests match existing endpoint behavior.

Frontend tests, where the project has coverage for similar forms:

- Duplicate action navigates to the add route with `duplicateFrom`.
- Add form fetches the draft when `duplicateFrom` is present.
- Add form submits through the existing create mutation.
- No create request is sent before the user submits.
- Draft warnings are visible.

## Acceptance Criteria

The feature is complete when:

1. Deployments and Stacks expose a `Duplicate` action.
2. Duplicate opens the existing add form with sanitized configuration.
3. The user can change the name, platform, tags, and configuration before saving.
4. No resource is created until form submission.
5. Runtime state, history, generated credentials, and resolved secret values are never copied.
6. Existing create validation and RBAC still apply.
7. Duplicate creation writes a resource-specific activity event on the new resource.
8. Generated frontend API types include the new draft endpoints.
