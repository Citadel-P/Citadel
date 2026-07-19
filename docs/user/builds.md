# Builds

Builds let Citadel build Docker images from Git repositories and push the generated tags to a configured registry.

Use builds when:

- the Dockerfile lives in a Git repository managed by Citadel
- you want operators to run image builds from Citadel
- the output image should be pushed to DockerHub, GitHub Container Registry, or a custom registry
- you need persisted run history, live logs, cancellation, and basic retention

Builds are not automation actions. Automation actions may call the Build API, but build projects do not run arbitrary TypeScript or shell scripts.

## Current Runner Support

Builds can run on Docker platforms connected through:

- `Local`
- `Agent`
- `EdgeAgent`

Local builds stream the build context from Citadel Core to the local Docker daemon.

Agent and edge-agent builds package the resolved build context in Citadel Core, send that archive to the selected agent, and let the agent stream it to its Docker daemon. The transferred archive must fit within the current agent message envelope limit of `16 MB`.

## Prerequisites

Before creating a build, configure:

- a Git repository that contains the Dockerfile and build context
- a Docker platform where the build can run
- a registry where Citadel can push the built image
- optional Citadel secrets for future BuildKit secret mounts

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
- Platform: Docker platform that runs the build.
- Registry: registry Citadel pushes tags to.
- Image repository: repository path under the selected registry, such as `team/api`.
- Tags: comma-separated tag templates.
- Update deployments: optional deployments that should use the successful build output.
- Enabled: whether runs can be queued.
- Timeout seconds: maximum build and push duration.
- Run retention: number of recent terminal runs to keep.
- Build arguments: optional JSON array of Docker build args.
- Build secrets: reserved JSON array of future BuildKit secret mounts.

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

## Build Context Size

The build context is the directory sent to Docker. Keep it small and explicit.

For local builds, a large context slows the Docker API upload and can make logs appear delayed.

For agent and edge-agent builds, Citadel first packages the context and transfers it to the agent. The packaged context must be under `16 MB`. If it is larger, the run fails before Docker starts.

Add a `.dockerignore` file in the context directory to exclude files that are not needed by the Dockerfile.

Common exclusions:

```text
.git
node_modules
bin
obj
dist
coverage
*.log
```

In monorepos, prefer the smallest service directory as the context instead of using the repository root.

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

## Deployment Consumers

Use **Update deployments** when a successful build should move one or more Citadel deployments to the newly built image.

Citadel updates only deployments that use an external image from the same registry configured on the build. On success, Citadel writes the built image reference and digest into the deployment spec, records a `DeploymentUpdated` activity event, and streams the deployment update to connected clients.

If a selected deployment no longer exists, has no deployment spec, uses a local image, or points at a different registry, the build still succeeds. Citadel writes a warning to the run log and skips that deployment.

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

Use build secrets for sensitive values once BuildKit secret sessions are supported by the runner.

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

Current limitation: the Docker Engine API runner does not support BuildKit secret sessions yet. If build secrets are configured, the run is rejected before Docker starts. Use build arguments only for non-sensitive values.

## Run A Build

Use **Build** from the build list or build detail page.

A run:

1. claims the queued run
2. syncs the Git repository branch
3. resolves the exact commit SHA
4. validates the context and Dockerfile paths
5. resolves build args, validates that build secrets are not configured, and resolves registry credentials
6. runs the Docker Engine API build on the selected platform
7. pushes all generated image tags to the registry
8. updates configured deployment consumers when the build succeeds
9. stores run status, image references, digest when available, and logs

Only one run per build project can be active at a time. If a run is already queued, preparing, or running, Citadel rejects another run for the same project.

## Webhooks

Enable webhooks on a build project when a Git provider should queue a build after a push.

Build webhook runs use the same configured repository, branch, context, Dockerfile, platform, registry, and tag templates as manual runs. Citadel validates the provider secret, branch, and repository identity, then queues the build with trigger `Webhook`.

Citadel uses changed paths from the webhook payload when the provider includes them. A build is queued only when a changed path is inside the configured build context or exactly matches the configured Dockerfile path.

If the payload does not include changed paths, Citadel syncs the repository branch, diffs the latest successful build commit against the new branch head, and applies the same path decision. This keeps monorepo builds from running when only unrelated services changed.

Only one run per build project can be active at a time. If a webhook arrives while a run is already queued, preparing, or running, Citadel rejects the delivery instead of starting another build.

For the shared listener URL format and provider setup, see `docs/user/webhooks.md`.

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

Citadel also uses the existing cleanup job to remove old terminal build runs. By default, completed build runs older than `90` days are deleted during the normal cleanup cycle. Administrators can configure this with:

```text
Builds:RunCleanupEnabled
Builds:RunRetentionDays
```

Retention does not delete:

- pushed registry images
- Git commits
- local platform images
- activity records required by the audit model

## Troubleshooting

If no platform appears in the build form:

- create a Docker platform
- confirm the platform is accessible from Citadel Core
- for agent and edge-agent platforms, confirm the agent is running a build-capable version

If branch discovery is empty:

- sync the Git repository
- confirm repository credentials are valid
- confirm the branch exists on the remote

If the build fails before Docker starts:

- check that Context exists in the repository
- check that Dockerfile exists
- confirm paths are relative to the repository root
- for agent and edge-agent builds, reduce the context below `16 MB` with `.dockerignore`
- confirm the selected registry has push credentials when required
- remove build secrets until BuildKit secret sessions are supported by the runner

If the push fails:

- confirm the registry host and image repository are correct
- confirm the registry token has push permission
- confirm the Citadel host can reach the registry

If secrets appear masked in logs, that is expected. Citadel redacts configured secret values before storing or streaming log output.
