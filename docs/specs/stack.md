# Stack

## Purpose

This spec covers general stack lifecycle behavior that is not specific to drift detection or manual auto-update.

## Main Files

- `src/Citadel.Domain/Entities/Stacks/Stack.cs`
- `src/Citadel.Domain/Entities/Stacks/StackRelease.cs`
- `src/Citadel.Application/Features.Stacks/Commands/PatchStack.cs`
- `src/Citadel.Application/Features.Stacks/Commands/RenameStack.cs`
- `src/Citadel.Application/Features.Stacks/Commands/DeleteStacks.cs`
- `src/Citadel.Application/Services/ApplyStackService.cs`
- `src/Citadel.FrontEnd/src/features/stacks/form/form.tsx`
- `test/Citadel.Tests.Unit/Application/Features/Stacks/StackLifecycleTests.cs`
- `test/Citadel.Tests.Integration/Application/Features.Stacks/StackPatchTests.cs`

## Stack Release Semantics

`StackRelease` rows represent apply/deploy attempts, not every form edit.

Stack form edits:

- update the current release definition in place
- do not create a new release row
- do not increment `Version`
- do not reset release status to `Created`

Stack apply:

- the first apply of a `Created` stack uses the initial version `1` release
- applying a stack that already has a non-`Created` release creates the next release version
- Citadel runtime labels must use the release id that is actually being applied

## Stack Rename And Delete

The Docker Compose project name is runtime identity, not just display metadata.

If a stack does not have an explicit `Spec.ProjectName`, Citadel derives one from `Stack.Name`. After the stack has been deployed, renaming `Stack.Name` must not change the Compose project name used for runtime lookup. Otherwise existing containers keep the old `com.docker.compose.project` label and runtime lookups report services as missing.

Rename behavior:

- before changing `Stack.Name`, resolve the current Compose project name
- if `CurrentStackRelease.Spec.ProjectName` is empty, pin it to the resolved project name
- then update the stack display name
- do not recreate containers as part of rename

Delete behavior:

- deleting a deployed stack should delete Citadel-owned Docker containers before deleting stack metadata
- container cleanup should use the resolved Compose project name and Citadel ownership labels
- unmanaged containers in the same Compose project must not be deleted
- volumes are not deleted by default
- if the platform is unavailable for a deployed stack, deletion should fail instead of silently orphaning runtime containers

## Tests

Current test coverage should include:

- stack form edits do not create releases or bump versions
- stack apply creates the next release version only after the initial created release has already been used
- renaming a deployed stack pins the previous Compose project name
- deleting a deployed stack deletes owned runtime containers before stack metadata
