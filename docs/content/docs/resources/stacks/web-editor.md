---
title: "Web Editor Stacks"
description: "Author and operate Stack Compose files directly in the Citadel editor."
---

Web Editor Stacks deploy Compose YAML stored in Citadel as a Docker Compose
project on a Standalone Platform or a native Stack on a Swarm Platform. Use
this when the application does not need a Git repository workflow.

Use deployments instead when the workload is a single Docker container and does not need Compose. See [Deployments](/docs/resources/deployments).

Use Git stacks instead when the Compose files should be reviewed, versioned, and deployed from a repository. See [Git Stacks](/docs/resources/stacks/git).

To import a Docker Compose project that is already running without applying it
again during onboarding, see [Adopt existing workloads](/docs/guides/adopting-existing-workloads).

## Deploy a Compose file

1. Open **Stacks** and select **Add Stack**.
2. Choose a name, an online Platform, **Web Editor** as the source, and a Registry such as **Docker Hub**.
3. Paste the application's Compose file and fix any reported errors.
4. Add referenced variables and secrets on the **Bindings** tab.
5. Select **Save**, then **Deploy**. Check the result on the **Services** tab.

Try [your first Stack](/docs/getting-started/first-stack) if you do not have a Compose file yet.
On Swarm, Citadel checks whether the Compose fields can be represented by native Swarm services.

## Compose example

This example publishes a web service on port `8080` and keeps its content in a
named volume:

```yaml
services:
  app:
    image: nginx:alpine
    ports:
      - "8080:80"
    volumes:
      - app_data:/usr/share/nginx/html

volumes:
  app_data:
```

Saving changes does not update the running application. Select **Deploy** or
**Redeploy** to apply them. Use [volume browsing](/docs/resources/docker/volumes#browse-and-download-files)
to inspect the named volume's contents.

## Compose File

The Compose editor stores one Compose YAML document in Citadel. It is the source of truth for a web editor stack.

Use Docker Compose syntax for Standalone Platforms. Swarm Stacks use Docker's
supported Compose v3 fields; Citadel checks compatibility before deployment.

- `services` define the containers Citadel deploys.
- `volumes` define named Docker volumes.
- `networks` define stack-specific or external networks.
- image tags should be explicit when you want predictable updates, for example `postgres:16` instead of `postgres:latest`.

Citadel injects ownership labels during apply. Do not manually define labels reserved by Citadel.

## Variables And Secrets

Use the stack `Bindings` tab for values that would normally live in a local `.env` file.

Compose file:

```yaml
services:
  app:
    image: ghcr.io/example/app:${IMAGE_TAG}
    environment:
      APP_ENV: ${APP_ENV}
      API_KEY: ${API_KEY}
```

Then define `IMAGE_TAG`, `APP_ENV`, and `API_KEY` on the stack `Bindings` tab.

The editor warns when a required `${NAME}` reference is not defined in stack or global bindings. At deploy time, Citadel resolves bindings and writes them to a generated env file for Docker Compose interpolation.

Generated env file path:

- Leave empty for the default temporary path.
- Set it only when the Compose project or scripts need a predictable env file path.
- The path is relative to the Docker Compose run directory.

For more detail, see [Variables and secrets](/docs/guides/variables-and-secrets).

## Update Behavior

On Docker Standalone and Swarm, Web Editor Stacks can check service image tags
for new digests when a registry is selected.

- `Disabled`: disable periodic checks; manual checks remain available.
- `Notify Only`: record image update availability and emit an alert.
- `Auto Deploy Services`: pull updated images and redeploy only changed services. Available for Docker Standalone only.
- `Auto Deploy Stack`: pull updated images and redeploy the full stack.

For registry setup, see [Registries](/docs/resources/registries).

Image update checks only work for services with image references that can be resolved as repository and tag pairs. Pinned digest-only images are not checked as tag updates.

Use `Notify Only` for production stacks until the stack has been tested. Use auto-deploy modes only when the image tag policy is controlled and rollback expectations are clear.

Update detection and `Notify Only` remain available in Community. Automatic
deployment caused by a detected image change requires Team's
`Operational Guardrails` and `Automated Operations` capabilities.

Use **Check for updates** on the Stack page to query the selected registry
immediately. The check records service-image update state but does not pull or
apply an image, emit an alert, or run an automatic deployment. It remains
available when periodic update behavior is disabled.

Citadel compares the registry digest with the deployed image digest, so the
first check can detect an update. On Standalone, it reads the Stack's containers
and local images; on Swarm, it reads the manager's Service image references.
If the deployed digest is unavailable, deploy the Stack with registry access
before checking again. Services whose images come from Citadel Builds are
excluded because their artifact state is managed by the Build workflow.

On the Stacks page, use **Updates available** to show only stacks with a
detected newer image digest. The filter works with search, tags, and the
Platform filter.

## Build Images

Use **Build Images** when one or more Compose services should use images produced by Citadel build projects.

For each service binding:

- Compose Service: exact service name from the Compose file
- Build: build project that produces the service image
- Redeploy On Build: automatically reapply that service after the selected build succeeds

`Redeploy On Build` requires Team's `Automated Operations` capability. Without
it, Citadel records the desired artifact and waits for a manual stack apply.

When the stack is applied, Citadel uses the desired artifact stored on each binding and injects that image into the generated Compose content. A binding without a resolved artifact falls back to the latest successful build. You can save bindings before the first successful build, but apply fails until each selected build has a successful image.

When a mapped build succeeds later, Citadel updates the binding's desired image reference and digest. If `Redeploy On Build` is enabled, Citadel reapplies only the mapped service. Applied state changes only after that apply succeeds.

Service-scoped deployment is supported only for Docker Standalone Stacks.
For a Swarm Stack, leave **Redeploy On Build** disabled and deploy the complete
Stack manually after the desired build image changes.

For build setup and webhook-triggered builds, see [Builds](/docs/resources/builds).

## Project Name

Docker Compose uses a project name to group containers, networks, and volumes.

By default, Citadel derives the Compose project name from the stack name. Set **Project Name** only when:

- importing or taking over an existing Compose project
- keeping the runtime project name stable while renaming the Citadel stack
- avoiding a collision with another Compose project on the same platform

Use the dedicated Compose import flow for an existing running project. It
preserves the detected project name, compares the supplied Compose source with
the running services, and associates the project without running Docker
Compose.

Project names must start with a lowercase letter or digit and contain only lowercase letters, digits, dashes, or underscores.

## Pre Deploy And Post Deploy

Use advanced commands when the stack needs host-side work around `docker compose up`.

Pre-deploy and post-deploy commands are available only for Docker Standalone
Stacks. They are hidden and rejected for Swarm Stacks.

Pre deploy examples:

- log in to a private registry
- generate a config file
- create a host directory for a bind mount

Post deploy examples:

- run a migration command
- seed initial data
- notify another system

The command path is relative to the run directory. Keep commands idempotent; they may run again during a reapply or rollback.

## Destroy Before Deploy

The **Destroy** option runs `docker compose down` before redeploying the stack.

This option is available only for Docker Standalone Stacks. Enable it when the
application requires a full shutdown before deployment and you accept the
downtime. Leave it disabled when the Compose project can be updated in place.

Service-scoped auto-deploy does not use destructive full-stack behavior.

## Drift Management

Drift management compares the Compose definition against containers currently running on the platform.

This feature applies to Docker Standalone Stacks. Swarm owns Task convergence
and does not expose these container drift controls.

- `Disabled`: do not check runtime drift.
- `Detect only`: detect drift, alert, and optionally mark the stack degraded.
- `Auto-fix safe drift`: allow safe fixes such as starting stopped containers or resuming paused containers.

Use drift detection when operators may change containers outside Citadel and you want Citadel to report those differences.

Manual drift checks remain available in Community. Continuous drift monitoring
and automatic safe reconciliation require Team's `Operational Guardrails`
capability.

The manual **Reconcile drift** action also requires Operational Guardrails and
an enabled drift policy. Inspect the report first: an intentionally stopped
container should not be restarted merely to clear a warning. See
[handling drift](/docs/guides/application-updates#handle-drift-separately).

## Rollback

Rollback uses a previously healthy release snapshot. For web editor stacks, the rollback snapshot includes the Compose content and stack settings from that release.

If the current Compose editor has newer changes, rollback does not deploy those edits. It deploys the selected release snapshot.

Rollback does not restore application volumes or historical secret values.
Bindings are resolved again, and mutable image tags can point to newer images.
Review those dependencies before rollback. Swarm Secret and Config limitations
are described in the [Swarm guide](/docs/resources/stacks/swarm#create-and-deploy-a-swarm-stack).

## When To Use Web Editor Stacks

Use web editor stacks for:

- small internal services
- quick Compose deployments
- stacks managed by Citadel administrators
- environments where Git integration is not available

Use Git stacks for:

- production GitOps workflows
- pull-request review before deployment
- monorepos
- webhook-triggered deploys from a Git provider
- pinning deployments to specific commits
