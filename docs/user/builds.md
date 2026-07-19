# Builds

Builds let Citadel build Docker images from Git repositories and push the generated tags to a configured registry.

Use builds when:

- the Dockerfile lives in a Git repository managed by Citadel
- you want operators to run image builds from Citadel
- the output image should be pushed to DockerHub, GitHub Container Registry, or a custom registry
- you need persisted run history, live logs, cancellation, and basic retention

Builds are not automation actions. Automation actions may call the Build API, but build projects do not run arbitrary TypeScript or shell scripts.

## Current Runner Support

Builds currently run only on `Local` Docker platforms.

Agent and edge-agent platforms can manage deployments, stacks, and Docker resources, but build execution on those connectors requires the build helper connector protocol. Citadel rejects build projects that select an agent or edge-agent platform until that support is implemented.

## Prerequisites

Before creating a build, configure:

- a Git repository that contains the Dockerfile and build context
- a local Docker platform where the build can run
- a registry where Citadel can push the built image
- optional Citadel secrets for BuildKit secret mounts

For repository setup, see `docs/user/git-repositories.md`.

For registry setup, see `docs/user/registries.md`.

For secrets, see `docs/user/variables-and-secrets.md`.

## Create A Build

Open:

```text
Builds -> Add Build
```

Set:

- Name: unique name shown in lists, activity, and run history.
- Description: optional notes for other admins.
- Tags: optional filters for organizing build projects.
- Repository: Git repository that contains the Dockerfile.
- Branch: branch Citadel syncs before building.
- Context: directory relative to the repository root sent to Docker as the build context.
- Dockerfile: Dockerfile path relative to the repository root.
- Target stage: optional Dockerfile stage for multi-stage builds.
- Platform: local Docker platform that runs the build.
- Registry: registry Citadel pushes tags to.
- Image repository: repository path under the selected registry, such as `team/api`.
- Tags: comma-separated tag templates.
- Enabled: whether runs can be queued.
- Timeout seconds: maximum build and push duration.
- Run retention: number of recent terminal runs to keep.
- Build arguments: optional JSON array of Docker build args.
- Build secrets: optional JSON array of BuildKit secret mounts.

Save the build before queueing a run.

## Source And Branches

The build source is an existing Citadel Git repository.

When a repository is selected, Citadel discovers branches and shows them in the branch field. If branch discovery fails, check repository credentials and sync status.

The context and Dockerfile paths are relative to the repository root:

```text
Context: .
Dockerfile: Dockerfile
```

For a monorepo:

```text
Context: services/api
Dockerfile: services/api/Dockerfile
```

Paths must stay inside the repository. Absolute paths and path traversal are rejected.

## Image Repository And Tags

The registry supplies the host. The image repository should not include a registry host, tag, or digest.

Examples:

```text
team/api
internal/jobs/report-worker
```

Tag templates support:

- `{branch}`: sanitized branch name
- `{shortSha}`: first 12 characters of the resolved commit
- `{sha}`: full resolved commit SHA

Common tag templates:

```text
{branch}-{shortSha}
{branch}, {branch}-{shortSha}
latest, {shortSha}
```

Citadel resolves the final image references when the run resolves the Git commit.

## Build Arguments

Build arguments are passed to Docker with `--build-arg`.

Example:

```json
[
  {
    "name": "NODE_ENV",
    "value": "production"
  }
]
```

Do not put passwords, tokens, or private keys in build arguments. Docker build args can leak into image history or metadata.

Use build secrets for sensitive values.

## Build Secrets

Build secrets map a BuildKit secret id to an existing Citadel secret.

Example:

```json
[
  {
    "id": "npmrc",
    "secretId": "019f0000-0000-7000-9000-000000000000"
  }
]
```

The Dockerfile consumes the secret with BuildKit syntax:

```dockerfile
RUN --mount=type=secret,id=npmrc \
    cp /run/secrets/npmrc ~/.npmrc && npm ci
```

Secret values are resolved only when a run starts. Citadel does not store secret values in build project snapshots, run snapshots, logs, or image references.

## Run A Build

Use **Build** from the build list or build detail page.

A run:

1. claims the queued run
2. syncs the Git repository branch
3. resolves the exact commit SHA
4. validates the context and Dockerfile paths
5. resolves build args, build secrets, and registry credentials
6. runs Docker BuildKit on the local platform
7. pushes all generated image tags to the registry
8. stores run status, image references, digest when available, and logs

Only one run per build project can be active at a time. If a run is already queued, preparing, or running, Citadel rejects another run for the same project.

## Logs And History

Open the **Runs** tab on a build project to see run history.

Use the log action on a run to view persisted logs. Active runs stream logs live while the sheet is open. Logs are stored with the run until retention removes that run.

Run statuses:

- `Queued`: waiting for the build worker.
- `Preparing`: worker claimed the run and is preparing source, secrets, and registry auth.
- `Running`: Docker build or push is active.
- `Succeeded`: build and push completed.
- `Failed`: build, push, validation, or setup failed.
- `TimedOut`: the run exceeded its timeout.
- `Cancelled`: a user cancelled the run.
- `Interrupted`: Citadel stopped or lost the active execution session.

## Cancellation

Use the cancel action on an active run.

Cancellation is best-effort:

- queued runs are marked cancelled before they start
- preparing or running runs receive a cancellation signal
- Citadel updates the build project back to idle after the run becomes terminal

If Citadel restarts during a run, the reconciler marks orphaned active runs as interrupted.

## Retention

Each build project keeps its most recent terminal runs according to **Run retention**.

When a new terminal run is recorded, Citadel deletes older terminal runs beyond the retention count. Persisted logs for deleted runs are removed with the run.

Retention does not delete:

- pushed registry images
- Git commits
- local platform images
- activity records required by the audit model

## Troubleshooting

If no platform appears in the build form:

- create a local Docker platform
- confirm the platform is accessible from Citadel Core
- agent and edge-agent platforms are not supported for builds yet

If branch discovery is empty:

- sync the Git repository
- confirm repository credentials are valid
- confirm the branch exists on the remote

If the build fails before Docker starts:

- check that Context exists in the repository
- check that Dockerfile exists
- confirm paths are relative to the repository root
- confirm the selected registry has push credentials when required
- confirm configured build secrets still exist

If the push fails:

- confirm the registry host and image repository are correct
- confirm the registry token has push permission
- confirm the Citadel host can reach the registry

If secrets appear masked in logs, that is expected. Citadel redacts configured secret values before storing or streaming log output.
