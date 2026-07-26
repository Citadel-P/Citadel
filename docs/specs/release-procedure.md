# Stable Release Procedure

## Scope

Citadel publishes stable semantic versions only. Release tags use the exact
`vMAJOR.MINOR.PATCH` form, for example `v1.0.0` or `v1.0.1`.

Public container images are published only by stable release tag workflows.
Pull requests, pushes to `main`, and scheduled nightly runs build and test
candidate images without pushing them to GHCR or Docker Hub.

Citadel Core and Citadel Agent are separate repositories with separate release
workflows. Release each repository independently. When a change affects both
applications, use the same product version in both repositories and release the
Agent before Core.

Do not create `alpha`, `beta`, `rc`, or other prerelease tags with the current
workflow.

## Version Files

The release version is defined in the root `version.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/dotnet/Nerdbank.GitVersioning/main/src/NerdBank.GitVersioning/version.schema.json",
  "version": "1.0.1",
  "assemblyVersion": "1.0.0.0",
  "release": {
    "tagName": "v{version}"
  },
  "publicReleaseRefSpec": [
    "^refs/tags/v\\d+\\.\\d+\\.\\d+$"
  ],
  "cloudBuild": {
    "buildNumber": {
      "enabled": true
    }
  }
}
```

- `version` is the product and Docker image version. Update it for every
  release.
- `assemblyVersion` is the .NET binary compatibility version. Do not change it
  for a patch release.
- A minor or major release requires an explicit compatibility review before
  changing `assemblyVersion`, the shared compatibility version, or the Agent
  protocol version.

Nerdbank.GitVersioning calculates versions from the committed `version.json`.
Commit a version change before validating it or creating the release tag.

## CI Builds

Nerdbank.GitVersioning still calculates commit-qualified versions for untagged
builds, but CI does not publish those builds to a public registry. Candidate
Docker images remain private workflow artifacts. Core is built once by the E2E
job, tested by browser smoke and accessibility checks, and reused by downstream
runtime and compatibility jobs.

| Workflow event | Build and test | Push to GHCR and Docker Hub |
| --- | --- | --- |
| Pull request | Yes | No |
| Push to `main` | Yes | No |
| Nightly schedule | Yes, including nightly runtime tests | No |
| Stable `vMAJOR.MINOR.PATCH` tag | Yes | Yes |

After releasing `1.0.0`, update `version.json` on `main` to the next intended
stable version, such as `1.0.1`. This keeps local binaries and CI diagnostics
associated with the next release even though no development image is published.

## Prepare The Release

Perform these steps in each repository being released.

Before the first release, configure these GitHub Actions values in both
repositories, preferably at the `Citadel-P` organization level:

| Type | Name | Value |
| --- | --- | --- |
| Variable | `DOCKERHUB_NAMESPACE` | `citadelplane` |
| Variable | `DOCKERHUB_USERNAME` | Docker Hub user or service account used to publish images |
| Secret | `DOCKERHUB_TOKEN` | Docker Hub access token with permission to push both images |

The public Docker Hub repositories are
[`citadelplane/citadel`](https://hub.docker.com/repository/docker/citadelplane/citadel)
and
[`citadelplane/citadel-agent`](https://hub.docker.com/repository/docker/citadelplane/citadel-agent).
Use an access token rather than a Docker Hub account password.

1. Update the local `main` branch and confirm the intended release changes are
   present.

   ```powershell
   git switch main
   git pull --ff-only
   git status
   ```

2. Update `version.json` to the exact release version. For `v1.0.1`:

   ```json
   "version": "1.0.1"
   ```

3. Review and commit the release changes.

   ```powershell
   git add version.json
   git commit -m "Release 1.0.1"
   ```

   Include other release changes in the same commit when appropriate. The tag
   must point to a commit that already contains the matching `version.json`.

4. Restore the pinned NBGV tool and verify the committed version:

   ```powershell
   dotnet tool restore
   dotnet nbgv get-version -v SimpleVersion
   ```

   The command must print the version without the `v` prefix:

   ```text
   1.0.1
   ```

5. Run the repository checks.

   Citadel Core:

   ```powershell
   dotnet build test/Citadel.Tests.Unit/Citadel.Tests.Unit.csproj -c Release
   dotnet build test/Citadel.Tests.Integration/Citadel.Tests.Integration.csproj -c Release
   dotnet run --project test/Citadel.Tests.Unit/Citadel.Tests.Unit.csproj --no-build -c Release
   dotnet run --project test/Citadel.Tests.Integration/Citadel.Tests.Integration.csproj --no-build -c Release
   npm ci --prefix src/Citadel.FrontEnd
   npm run build:prod --prefix src/Citadel.FrontEnd
   ```

   Citadel Agent:

   ```powershell
   dotnet build Citadel.Agent.slnx -c Release
   dotnet run --project test/Citadel.Agent.Tests.Unit/Citadel.Agent.Tests.Unit.csproj --no-build -c Release
   ```

## Create The Release

Create an annotated tag on the verified release commit:

```powershell
git tag -a v1.0.1 -m "Release v1.0.1"
git push origin main
git push origin v1.0.1
```

The workflow rejects a tag when:

- The tag is not in the exact `vMAJOR.MINOR.PATCH` form.
- The tag does not match the committed `version.json`.
- NBGV does not recognize the tag as a public release.
- The build, test, frontend, or Docker image checks fail.
- A required acceptance job does not pass against the archived candidate.
- Any published tag does not resolve to the tested candidate config digest and
  the same immutable manifest digest.

Do not move or reuse a published release tag.

The release workflow does not wait for the separately scheduled nightly runtime
suite. Run or review the latest nightly workflow before tagging when a release
requires that additional confidence.

## Published Images

A successful `v1.0.1` release publishes:

```text
ghcr.io/citadel-p/citadel:1.0.1
ghcr.io/citadel-p/citadel:1.0
ghcr.io/citadel-p/citadel:1
docker.io/citadelplane/citadel:1.0.1
docker.io/citadelplane/citadel:1.0
docker.io/citadelplane/citadel:1
```

The Agent repository publishes the corresponding tags under:

```text
ghcr.io/citadel-p/citadel.agent:1.0.1
ghcr.io/citadel-p/citadel.agent:1.0
ghcr.io/citadel-p/citadel.agent:1
docker.io/citadelplane/citadel-agent:1.0.1
docker.io/citadelplane/citadel-agent:1.0
docker.io/citadelplane/citadel-agent:1
```

The exact patch tag is immutable. The minor and major aliases move to the most
recent stable release in that release line. Citadel does not publish a `latest`
tag. The workflow promotes the exact candidate that passed the required checks;
it does not rebuild Core for publication. It records the candidate config digest
and published manifest digest in the workflow summary, then signs that manifest
in both registries.

## Verify The Release

1. Confirm that the repository's `Docker` workflow completed successfully.
2. Confirm that the exact image tag exists in GHCR and Docker Hub.
3. Confirm the GHCR and Docker Hub exact tags resolve to the manifest digest
   recorded in the workflow summary.
4. Pull each exact image tag, not a floating alias, for release verification.
5. Confirm that the Citadel UI reports the expected product version.
6. For a coordinated release, confirm that Agent connections report a
   compatible version.

## Rollback And Corrections

Rollback deployments by selecting the previous exact image tag, for example
`1.0.0`. Do not move `v1.0.1` or overwrite the `1.0.1` image.

If a published release is defective, fix the issue and publish a new patch
release, such as `v1.0.2`. The `1.0` and `1` aliases will move to that release
after its workflow succeeds.
