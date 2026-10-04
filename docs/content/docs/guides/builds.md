---
title: "Builds"
description: "Build container images from Git repositories and publish them through Citadel."
---

Builds let Citadel build Docker images from Git repositories and push the generated tags to a configured registry.

Use builds when:

- the Dockerfile lives in a Git repository managed by Citadel
- you want operators to run image builds from Citadel
- the output image should be pushed to DockerHub, GitHub Container Registry, or a custom registry
- you need persisted run history, live logs, cancellation, and basic retention

Builds are not automation actions. Automation actions may call the Build API, but build projects do not run arbitrary TypeScript or shell scripts.

## Build your first image

You need a synced [repository](/docs/guides/git-repositories), a Dockerfile,
an online builder Platform, and a [registry](/docs/guides/registries) that can
accept image pushes. Ask the application's maintainer for the build paths.

1. Open **Build Projects** and select **Add**.
2. Choose the repository and branch, Dockerfile path, and build context.
3. Select the builder Platform, destination Registry, image repository, and tag.
4. Save the project and start a manual build.
5. Check the run's logs and published image before using **Build** as an application's image source.

A successful build creates an image; it does not start an application by itself.
You can begin with a connected Platform instead of setting up a Build Pool.

## License Availability

Community can:

- create build projects and build-pool definitions
- test and manage build-pool configuration
- run builds manually on existing Local, Agent, and Edge Agent platforms
- view build configuration, history, artifacts, and logs
- cancel in-flight builds

Team capabilities add:

- `Automated Operations`: webhook-triggered builds and automatic deployment or
  stack apply after a successful build
- `Elastic Build Execution`: builds dispatched through an external Build Pool

A webhook-triggered build using an external Build Pool requires both
capabilities. A manual build using an external Build Pool requires only
`Elastic Build Execution`.

If a license changes, an external build that has already started may finish or
be cancelled. Queued external-pool builds do not lease or provision new builder
capacity without `Elastic Build Execution`.

## Current Runner Support

Builds can run on Docker platforms connected through:

- `Local`
- `Agent`
- `EdgeAgent`
- `Build Pool`

Local builds stream the build context from Citadel Core to the local Docker daemon.

Agent and edge-agent builds package the resolved build context in Citadel Core, send that archive to the selected agent, and let the agent stream it to its Docker daemon. The transferred archive must fit within the current agent message envelope limit of `16 MB`.

Build Pool builds run on a reusable build pool instead of a Docker platform. For a self-managed VM pool, Citadel either connects to a dedicated inbound Agent endpoint or uses a pool-scoped Edge Agent connection. In both modes, the builder host's Docker daemon performs the build and push.

Executing a build through a Build Pool requires `Elastic Build Execution`.
Creating, testing, updating, disabling, or deleting the pool definition does
not.

## Prerequisites

Before creating a build, configure:

- a Git repository that contains the Dockerfile and build context
- a Docker platform or Build Pool where the build can run
- a registry where Citadel can push the built image
- optional Citadel secrets when your Dockerfile uses BuildKit secret mounts

For repository setup, see [Git repositories and accounts](/docs/guides/git-repositories).

For registry setup, see [Registries](/docs/guides/registries).

For secrets, see [Variables and secrets](/docs/concepts/variables-and-secrets).

For Agent and self-managed Build Pool setup, see [Regular Agent](/docs/operations/agent).

## Create A Build

Open:

```text
Build Projects -> Add
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
- Builder: Docker platform or Build Pool that runs the build.
- Registry: registry Citadel pushes tags to.
- Image repository: repository path under the selected registry, such as `team/api`.
- Tags: comma-separated tag templates.
- Enabled: whether runs can be queued.
- Timeout seconds: maximum build and push duration.
- Run retention: number of recent terminal runs to keep.
- Build arguments: optional Docker build args for non-sensitive values.
- Build secrets: optional mappings from Dockerfile BuildKit secret ids to Citadel secrets.

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

## Build Pools

Use a Build Pool when builds should run on builder infrastructure rather than on one of the Docker platforms managed by Citadel.

For a self-managed VM pool, choose one connection mode.

### Inbound Agent endpoint

Use this when Citadel Core can reach the builder host directly. Run a dedicated Citadel Agent on the builder host:

```bash
docker run -d \
  --name citadel-agent-build \
  --restart=always \
  -p 9001:9000 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HUB_PUBLIC_KEY="..." \
  ghcr.io/citadel-p/citadel.agent:1.2.3
```

Then create the pool:

```text
Build Pools -> Add Build Pool
Provider: Self-managed VM / Static VM
Connection mode: Inbound Agent endpoint
Endpoint: http://<builder-host>:9001
```

For local Docker testing where Citadel Core also runs in Docker, use:

```text
http://host.docker.internal:9001
```

Click **Test** on the Build Pool before selecting it in a build project. A ready pool confirms that Citadel can reach the Agent and that the Agent can reach Docker.

### Edge Agent

Use this when the builder host should connect outbound to Citadel Core and should not expose a build Agent port.

Create and save the build pool first:

```text
Build Pools -> Add Build Pool
Provider: Self-managed VM / Static VM
Connection mode: Edge Agent
```

After the pool is saved, open the pool's configuration and generate an Edge Agent enrollment token from the **Edge Agent enrollment** section. Citadel shows a Docker command for that build pool. Run it on the builder host.

The build pool Edge Agent is scoped to the build pool itself. You do not need to create a Platform resource, and there is no Edge Agent platform dropdown for build pools.

The generated command uses the Edge Agent gRPC endpoint. Its container port is
`8001`; the supplied Compose host mapping is `127.0.0.1:18001`. A remote builder
needs a reachable, secured gRPC address rather than the browser port or a local-only address.

Click **Test** on the Build Pool before selecting it in a build project. A ready edge pool confirms that the pool-scoped Edge Agent is connected, advertises build capabilities, and can reach Docker.

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

## Using Build Output

Build projects produce image references. Deployments and stacks consume those image references from their own forms.

### Deployments

In a deployment, choose:

- Image Source: `Build`
- Build: the build project that produces the image
- Redeploy On Build: whether Citadel should redeploy this deployment automatically after a successful build

`Redeploy On Build` requires `Automated Operations`. Without it, Citadel still
records the new desired artifact and shows it as pending, but an operator must
apply the deployment manually.

The deployment form shows the latest build artifact, the artifact currently desired by the deployment, and the artifact last applied successfully. You can save the deployment before the first successful build, but the deployment cannot be applied from that build image until a run succeeds.

When a build succeeds, Citadel updates each deployment that uses that build image source with the new desired image reference and digest, records a `DeploymentUpdated` activity event, streams the deployment update to connected clients, and writes the update to the build run log. If `Redeploy On Build` is enabled, Citadel redeploys the deployment after the build finishes.

Apply uses the stored desired artifact, not whichever build happens to be latest when apply starts. Citadel pins the runtime reference to the image digest when available and advances the applied fields only after the deployment starts successfully. A failed redeploy therefore remains visibly pending and does not claim that the new build is running.

### Stacks

In a web editor stack or Git stack, use **Build Images** to map Compose services to build projects.

Each binding contains:

- Compose Service: exact Compose service name, such as `api`
- Build: the build project that produces that service image
- Redeploy On Build: whether Citadel should redeploy that service after a successful build

`Redeploy On Build` requires `Automated Operations`. Without it, Citadel records
the desired artifact but does not automatically apply the stack.

When the stack is applied, Citadel uses the desired artifact stored on each binding. A binding without a resolved artifact falls back to the latest successful build. For web editor stacks, Citadel replaces the service image in the generated Compose content. For Git stacks, Citadel writes a generated Compose override file with the resolved image references.

When a build succeeds, Citadel updates matching stack build image bindings with the new desired image reference and digest, records a `StackUpdated` activity event, streams the stack update to connected clients, and writes the affected service names to the build run log. If `Redeploy On Build` is enabled, Citadel reapplies only the mapped services. Applied provenance changes only for services that were included in a successful apply.

If a mapped build has no successful image yet, save is allowed, but stack apply fails until that build has a successful run.

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

Store sensitive values as Citadel secrets and reference them from Build secrets instead of typing raw secret values in the build form.

## Build Secrets

Build secrets map a BuildKit secret id to an existing Citadel secret. The BuildKit id must match the `id` used by the Dockerfile, and the value must come from a Citadel secret.

Build secrets are not raw `name/value` pairs. Configure them by selecting an existing Citadel secret from the build form. The text id you enter is only the Dockerfile mount id, for example `npmrc`, `pip_index_url`, or `github_token`.

The Dockerfile consumes the secret with BuildKit syntax:

```dockerfile
RUN --mount=type=secret,id=npmrc \
    cp /run/secrets/npmrc ~/.npmrc && npm ci
```

Secret values are resolved only when a run starts and are served to the Docker daemon through a BuildKit session. Citadel does not store secret values in build project snapshots, run snapshots, logs, or image references.

Each resolved build secret must be no larger than `60 KiB`. Larger values are rejected before Docker starts. Do not move sensitive values to build arguments as a workaround.

## Run A Build

Use **Build** from the build list or build detail page.

A run:

1. claims the queued run
2. syncs the Git repository branch
3. resolves the exact commit SHA
4. validates the context and Dockerfile paths
5. resolves build args, resolves Citadel-backed build secret mappings, and resolves registry credentials
6. runs the Docker Engine API build on the selected platform
7. pushes all generated image tags to the registry
8. updates deployment and stack build-image consumers when the build succeeds
9. redeploys consumers that have `Redeploy On Build` enabled
10. stores run status, image references, digest when available, and logs

Step 9 requires `Automated Operations`. Without it, the desired artifacts in
step 8 remain pending until an operator applies the consumer manually.

Only one run per build project can be active at a time. If a run is already queued, preparing, or running, Citadel rejects another run for the same project.

## Webhooks

Webhook-triggered builds require `Automated Operations`. Builds using an
external Build Pool additionally require `Elastic Build Execution`.

Enable webhooks on a build project when a Git provider should queue a build after a push.

Build webhook runs use the same configured repository, branch, context, Dockerfile, platform, registry, and tag templates as manual runs. Citadel validates the provider secret, branch, and repository identity, then queues the build with trigger `Webhook`.

Citadel uses changed paths from the webhook payload when the provider includes them. A build is queued only when a changed path is inside the configured build context or exactly matches the configured Dockerfile path.

If the payload does not include changed paths, Citadel syncs the repository branch, diffs the latest successful build commit against the new branch head, and applies the same path decision. This keeps monorepo builds from running when only unrelated services changed.

Only one run per build project can be active at a time. If a webhook arrives while a run is already queued, preparing, or running, Citadel rejects the delivery instead of starting another build.

**Build Webhook Received** activities show the delivery outcome:

- **Success**: the build was queued. This does not mean the build has succeeded.
- **Information**: an expected skip, such as no relevant changes, no new commit,
  a filtered branch or event type, or a disabled build project.
- **Warning**: a build could not be queued because the repository did not match,
  a required entitlement was unavailable, another run was active, or the
  configuration changed during dispatch. Other unexplained skips are warnings too.
- **Failure**: authentication, request validation, or dispatch failed.

The activity summary includes the outcome reason.

For the shared listener URL format and provider setup, see [Webhooks](/docs/guides/webhooks).

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

## Build pool connection and availability

For Edge Agent pools, the pool list shows the agent connection status separately
from the latest Docker build capability check. Connection changes update through
realtime notifications. The pool activity history records one **Connected** or
**Disconnected** event per transition, including disconnects detected by heartbeat
expiry and binding revocation.

The **Build Pool Unavailable** system rule raises a warning after an enabled
self-managed pool fails its capability checks for at least 90 seconds. This covers
both a disconnected agent and a connected agent whose Docker builder is unusable.
Checks run periodically, so an alert can appear after the grace period on the next
check. The outage start is retained across Core restarts.

The alert resolves when a capability check succeeds. Disabled, archived, and
never-connected Edge pools do not raise availability alerts. Disabling or archiving
a pool clears its availability incident. Configure notification channels on the
rule; advanced alerting also allows changing the grace period and limiting the
rule to selected build pools.

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
- for build-pool Edge Agent builds, confirm the pool's Edge Agent is connected and uses a build-capable Agent version
- confirm the selected registry has push credentials when required
- if build secrets are configured, confirm each BuildKit id maps to an existing Citadel secret selected in the build form
- if build secrets are configured, confirm each resolved secret is no larger than `60 KiB`

If Docker reports `NotFound: secret not found` while resolving the build graph, one of the Dockerfile `RUN --mount=type=secret,id=...` entries does not have a matching build secret mapping, or the selected Citadel secret is no longer available to the run.

If the push fails:

- confirm the registry host and image repository are correct
- confirm the registry token has push permission
- confirm the Citadel host can reach the registry

If a deployment or stack does not update after a successful build:

- confirm the deployment uses Image Source `Build`, not `External` or `Local`
- confirm the stack has a Build Images binding for the exact Compose service name
- confirm the selected build project is the one that just succeeded
- confirm `Redeploy On Build` is enabled if you expected an automatic redeploy
- check the build run log for consumer update messages

A queued build keeps the build settings selected when it was queued. Editing
the project does not change a build that is already waiting to run.

A successful build and a successful redeployment are separate outcomes. If the
image builds successfully but Apply fails, the build remains successful; check
the Deployment or Stack activity for the Apply error. For Stack bindings with
**Redeploy On Build**, only the selected services are redeployed.

If secrets appear masked in logs, that is expected. Citadel redacts configured secret values before storing or streaming log output.

