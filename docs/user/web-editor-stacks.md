# Web Editor Stacks

Web editor stacks let Citadel deploy a Docker Compose project from Compose YAML stored directly in Citadel. Use this when the stack is small, managed by Citadel operators, or does not need a Git repository workflow.

Use deployments instead when the workload is a single Docker container and does not need Compose. See `docs/user/deployments.md`.

Use Git stacks instead when the Compose files should be reviewed, versioned, and deployed from a repository. See `docs/user/git-stacks.md`.

## Basic Setup

Create a stack and choose:

- Source: `Web Editor`
- Platform: the Docker platform where the stack should run
- Compose file: the Docker Compose YAML for this stack
- Registry: the registry Citadel should use when checking image updates
- Update behavior: how Citadel should react to image updates

Example Compose file:

```yaml
services:
  app:
    image: nginx:1.27
    ports:
      - "8080:80"
    volumes:
      - app_data:/usr/share/nginx/html

volumes:
  app_data:
```

After saving the stack, use **Apply** to deploy it. Editing the Compose file changes the stack definition, but the platform is not changed until the stack is applied again.

## Compose File

The Compose editor stores one Compose YAML document in Citadel. It is the source of truth for a web editor stack.

Use standard Docker Compose syntax:

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

For more detail, see `docs/user/variables-and-secrets.md`.

## Update Behavior

Web editor stacks can check service image tags for new digests when a registry is selected.

- `Disabled`: do not check this stack for image updates.
- `Notify Only`: record image update availability and emit an alert.
- `Auto Deploy Services`: pull updated images and redeploy only changed services.
- `Auto Deploy Stack`: pull updated images and redeploy the full stack.

For registry setup, see `docs/user/registries.md`.

Image update checks only work for services with image references that can be resolved as repository and tag pairs. Pinned digest-only images are not checked as tag updates.

Use `Notify Only` for production stacks until the stack has been tested. Use auto-deploy modes only when the image tag policy is controlled and rollback expectations are clear.

## Build Images

Use **Build Images** when one or more Compose services should use images produced by Citadel build projects.

For each service binding:

- Compose Service: exact service name from the Compose file
- Build: build project that produces the service image
- Redeploy On Build: automatically reapply that service after the selected build succeeds

When the stack is applied, Citadel resolves each binding to the latest successful image from the selected build and injects that image into the generated Compose content. You can save bindings before the first successful build, but apply fails until each selected build has a successful image.

When a mapped build succeeds later, Citadel updates the stored binding with the new image reference and digest. If `Redeploy On Build` is enabled, Citadel reapplies only the mapped service.

For build setup and webhook-triggered builds, see `docs/user/builds.md`.

## Project Name

Docker Compose uses a project name to group containers, networks, and volumes.

By default, Citadel derives the Compose project name from the stack name. Set **Project Name** only when:

- importing or taking over an existing Compose project
- keeping the runtime project name stable while renaming the Citadel stack
- avoiding a collision with another Compose project on the same platform

Project names must start with a lowercase letter or digit and contain only lowercase letters, digits, dashes, or underscores.

## Pre Deploy And Post Deploy

Use advanced commands when the stack needs host-side work around `docker compose up`.

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

Keep it enabled for simple stacks and stacks that do not support rolling updates. Disable it only when the stack can be safely updated in place and you want to avoid tearing down services before every full reapply.

Service-scoped auto-deploy does not use destructive full-stack behavior.

## Drift Management

Drift management compares the Compose definition against containers currently running on the platform.

- `Disabled`: do not check runtime drift.
- `Detect only`: detect drift, alert, and optionally mark the stack degraded.
- `Auto-fix safe drift`: allow safe fixes such as starting stopped containers or resuming paused containers.

Use drift detection when operators may change containers outside Citadel and you want Citadel to report those differences.

## Rollback

Rollback uses a previously healthy release snapshot. For web editor stacks, the rollback snapshot includes the Compose content and stack settings from that release.

If the current Compose editor has newer changes, rollback does not deploy those edits. It deploys the selected release snapshot.

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
