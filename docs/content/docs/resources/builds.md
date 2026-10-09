---
title: "Build Projects"
description: "Build container images from Git repositories and publish them through Citadel."
---

Build Projects turn source from a Git repository into Docker images. A project
defines the source, builder, image tags, and optional Registry destination;
each run records its commit, output, and logs.

Use a Build Project for Dockerfile builds. Use
[Automation Actions](/docs/resources/automation-actions) for scripts that call
Citadel APIs or coordinate operational tasks.

## Build your first image

You need a synced [repository](/docs/resources/git-repositories), a Dockerfile,
an online builder Platform, and a [registry](/docs/resources/registries) that can
accept image pushes. Ask the application's maintainer for the build paths.

1. Open **Build Projects**, select **Add Build**, and enter a name.
2. Choose the repository and branch. For a repository with a Dockerfile at its
   root, set **Context** to `.` and **Dockerfile** to `Dockerfile`.
3. Under **Builder**, select **Docker platform** and an online Platform. Check
   [context limits](#build-context-size) before choosing a remote builder.
4. Enable **Push to registry**, select the Registry, and enter an **Image repository**
   such as `team/api`. Use `{branch}-{shortSha}` as the tag template to identify
   the source revision. The Registry supplies the hostname.
5. Save, select **Build**, and open **Runs**. Wait for **Succeeded**, then check
   the resolved commit, image reference, digest if available, and run logs.

The image is now available in the Registry. A Build Project does not start an
application until a Deployment or Stack uses its output.

### Deploy the image

1. Create or open a [Deployment](/docs/resources/deployments) on a Docker Standalone Platform.
2. Choose **Build** as its image source and select this Build Project. Configure
   the application's networks, ports, storage, and bindings.
3. Leave **Redeploy On Build** disabled for the first deployment. Save, then
   select **Deploy** or **Redeploy**.
4. Check the container's logs and application endpoint. Confirm the applied
   artifact matches the build you intended to run.

For Compose applications, use [Stack Build Images bindings](#stacks) instead.
The target host must be able to pull the published image with the configured
Registry access.

### Automate later builds

After the manual path works, enable a [build webhook](#webhooks) to build on
relevant Git changes and **Redeploy On Build** on the intended consumers. These
triggers require **Automated Operations**; external Build Pool execution also
requires **Elastic Build Execution**.

Push a small application change and verify both the Build run and the resulting
Deployment or Stack operation. Build success and deployment success are separate
outcomes. Use this built-in flow when it fits; an Automation script is only needed
for additional coordination. A queued build response does not mean its image is
ready to deploy.

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

Agent and Edge Agent builds package the committed context on Core and transfer
it to the builder. The current archive step is limited to **12 MiB**, before
the **16 MiB** transport envelope; see [Build Context Size](#build-context-size).

Build Pool builds run on a reusable build pool instead of a Docker platform. For a self-managed VM pool, Citadel either connects to a dedicated inbound Agent endpoint or uses a pool-scoped Edge Agent connection. In both modes, the builder host's Docker daemon performs the build and push.

Executing a build through a Build Pool requires `Elastic Build Execution`.
Creating, testing, updating, disabling, or deleting the pool definition does
not.

## Prerequisites

Connect the [Git repository](/docs/resources/git-repositories), an online
[Platform](/docs/resources/platforms) or [Build Pool](/docs/resources/build-pools),
and a [Registry with push access](/docs/resources/registries). Prepare any
[stored secrets](/docs/guides/variables-and-secrets) required by the Dockerfile.
Keep Git credentials, Registry credentials, and Dockerfile secret mounts separate:
they authorize different parts of the build.

## Create A Build

Open:

```text
Build Projects -> Add Build
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
- Push to registry: publish the output; disable only when an image local to the builder is sufficient.
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

For Agent, Edge Agent, and remote Build Pool execution, Core creates a Git tar
archive of the selected context with a **12 MiB** output limit. This happens
before transfer through the 16 MiB Agent envelope. An oversized archive fails
before Docker starts, and the Dockerfile must be inside the selected context.

The archive step does not apply `.dockerignore`. Reduce the selected context or
remove unnecessary tracked files to reduce the transfer size. A `.dockerignore`
file alone does not fix an oversized remote archive.

Use `.dockerignore` to control the files Docker needs for a local directory build.

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

To run builds on dedicated builder infrastructure, configure a [Build Pool](/docs/resources/build-pools)
and select it as the project's runner. The pool documentation covers inbound
Agent and Edge Agent setup, connection status, and availability alerts.

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

### Build without publishing

Disable **Push to registry** to keep the output on the selected builder. Registry
and image-repository settings are then cleared. The run records its local image
references, but another Platform cannot pull those images automatically. Builder
cleanup can remove them, especially on ephemeral infrastructure. Keep publishing
enabled for the build-to-deployment workflow above.

## Using Build Output

Build projects produce image references. Deployments and stacks consume those image references from their own forms.

### Deployments

In a deployment, choose:

- Image Source: `Build`
- Build: the build project that produces the image
- Redeploy On Build: whether Citadel should redeploy this deployment automatically after a successful build

`Redeploy On Build` requires `Automated Operations`. Without it, Citadel still
records the new desired artifact and shows it as pending, but an operator must
select **Deploy** or **Redeploy** manually.

The deployment form shows the latest build artifact, the artifact currently desired by the deployment, and the artifact last applied successfully. You can save the deployment before the first successful build, but the deployment cannot be applied from that build image until a run succeeds.

When a build succeeds, Citadel updates each deployment that uses that build image source with the new desired image reference and digest, records a `DeploymentUpdated` activity event, streams the deployment update to connected clients, and writes the update to the build run log. If `Redeploy On Build` is enabled, Citadel redeploys the deployment after the build finishes.

Deployment uses the saved desired artifact. A newer build arriving during deployment does not change the image being deployed. Citadel pins the image digest when available and records it as applied only after the deployment starts successfully. A failed redeploy leaves the desired artifact pending.

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

This service-scoped redeployment is supported only for Docker Standalone
Stacks. For a Swarm Stack, leave **Redeploy On Build** disabled and deploy the
complete Stack manually after the desired build image changes.

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
RUN --mount=type=secret,id=npmrc,target=/root/.npmrc \
    npm ci
```

This example assumes the install step runs as root. Adjust the mount path and
ownership for another build user. Keep the credential on the temporary mount;
copying it into the image filesystem can persist it in an image layer. See
[Docker's secret mounts](https://docs.docker.com/build/building/secrets/#secret-mounts).

Citadel resolves stored secrets during execution and passes them to BuildKit.
Keep values small and do not print them from the Dockerfile. Log redaction cannot
prevent a Dockerfile from writing a credential into the output image.

## Run A Build

Use **Build** from the build list or build detail page.

A run:

1. claims the queued run
2. syncs the Git repository branch
3. resolves the exact commit SHA
4. validates the context and Dockerfile paths
5. resolves build args, resolves Citadel-backed build secret mappings, and resolves registry credentials
6. builds the image on the selected builder
7. pushes generated image tags when **Push to registry** is enabled
8. updates deployment and stack build-image consumers when the build succeeds
9. redeploys supported consumers that have `Redeploy On Build` enabled
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
- `Succeeded`: build completed, including push when enabled.
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
- for remote builds, keep the committed context tar within the 12 MiB archive limit; `.dockerignore` is not applied by the archive step
- for build-pool Edge Agent builds, confirm the pool's Edge Agent is connected and uses a build-capable Agent version
- confirm the selected registry has push credentials when required
- if build secrets are configured, confirm each BuildKit id maps to an existing Citadel secret selected in the build form

If Docker reports `NotFound: secret not found` while resolving the build graph, one of the Dockerfile `RUN --mount=type=secret,id=...` entries does not have a matching build secret mapping, or the selected Citadel secret is no longer available to the run.

If the push fails:

- confirm the registry host and image repository are correct
- confirm the registry token has push permission
- confirm the selected builder can reach the registry

If a deployment or stack does not update after a successful build:

- confirm the deployment uses Image Source `Build`, not `External` or `Local`
- confirm the stack has a Build Images binding for the exact Compose service name
- confirm the selected build project is the one that just succeeded
- confirm `Redeploy On Build` is enabled if you expected an automatic redeploy
- check the build run log for consumer update messages

A queued build keeps the build settings selected when it was queued. Editing
the project does not change a build that is already waiting to run.

A successful build and a successful redeployment are separate outcomes. If the
image builds successfully but deployment fails, the build remains successful; check
the Deployment or Stack activity for the deployment error. For Stack bindings with
**Redeploy On Build**, only the selected services are redeployed.

If secrets appear masked in logs, that is expected. Citadel redacts configured secret values before storing or streaming log output.
