---
title: "Deployments"
description: "Create and operate Citadel Deployments for single-container workloads."
---

Deployments let Citadel run and manage a single Docker container on a selected platform. Use a deployment when one image is enough and you do not need a multi-service Docker Compose project.

Use a stack instead when the workload needs multiple services, Compose networks, Compose volumes, or Git-backed Compose configuration.

To bring an existing standalone Docker container under Citadel management
without recreating it first, see
[Adopt existing workloads](/docs/guides/adopting-existing-workloads).

## Run one container

You need an online Docker Standalone Platform and an image name.

1. Open **Deployments**, select **Add Deployment**, and choose a name and Platform.
2. Choose **External** and a Registry, then enter the image and tag.
3. Select at least one existing **Network**, then add the application's required ports and persistent storage.
4. Select **Save**, then **Deploy**. Wait for the operation to finish.
5. Open the Deployment to check its container and logs.

For a complete example, follow [your first Deployment](/docs/getting-started/first-deployment).
The sections below explain the other settings. Saving alone does not update the running container.

## Image Source

External images are pulled from a registry during deploy.

Use this for normal deployments:

- Registry: the configured Citadel registry to pull from
- Image reference: the image and tag, for example `nginx:1.27` or `ghcr.io/example/api:1.4.2`

For registry setup, see [Registries](/docs/resources/registries).

Local images are selected from images that already exist on the target Docker host.

Use local images when:

- the image was built directly on the platform
- the image is loaded by another process
- the platform cannot pull the image from a registry

Build images are produced by Citadel build projects.

Use this when the deployment should run images produced by a build:

- Image Source: `Build`
- Build: the build project that produces the image
- Redeploy On Build: automatically redeploy this deployment after the selected build succeeds

`Redeploy On Build` requires Team's `Automated Operations` capability. Without
it, a successful build advances the desired artifact, but an operator must
deploy it manually.

You can save a deployment before the selected build has a successful run. The form shows latest, desired, and applied artifacts. A successful build advances desired state; a successful deploy advances applied state. Deploy or redeploy succeeds only after the build has produced an image reference.

Citadel deploys the stored desired artifact and pins it to its digest when available. It falls back to the latest successful build only when the deployment has not resolved an artifact yet.

For build setup and webhook-triggered builds, see [Builds](/docs/resources/builds).

Auto update is only available for external image sources. It is disabled for local images and for external images pinned by digest, such as `nginx@sha256:...`.

Build images use `Redeploy On Build` instead of registry auto update.

## Networks

Select the Docker networks the container should join. The networks must already exist on the target platform.

To create one first, follow [Networks](/docs/resources/docker/networks).

Use the same network for containers that need to communicate with each other. Use a public or proxy network when a reverse proxy needs to reach the deployment.

## Ports

Ports publish container ports on the Docker host.

Examples:

```text
8080:80/tcp
127.0.0.1:5432:5432/tcp
```

For local images, Citadel can detect exposed image ports and pre-fill mappings. Review the mappings before deploying; exposed image metadata does not always match the ports you want public on the host.

Only publish ports that must be reachable outside Docker networks. For internal service-to-service traffic, prefer Docker networks without host port publishing.

## Volumes

Volumes mount persistent storage or host paths into the container.

Examples:

```text
app_data:/data
/srv/app/config:/etc/app:ro
```

Use Docker named volumes for application data you want Citadel to track for backup policies.

See [Volumes](/docs/resources/docker/volumes) to create storage or browse its contents.

Deployment storage policies include explicitly named volume sources and supported bind mounts discovered from the synchronized container, such as `/srv/app/config:/etc/app`. Host-path backups require Platform write access.

## Container Variables

Use **Container Variables** to inject environment variables into the container.

You can either expose a matching binding by name:

```text
APP_ENV
POSTGRES_PASSWORD
```

Or map one runtime environment key to another Citadel binding:

```text
DATABASE_PASSWORD=${POSTGRES_PASSWORD}
PUBLIC_URL=${APP_PUBLIC_URL}
```

Define referenced values on the deployment `Bindings` tab or as global bindings. At deploy time, Citadel resolves only the referenced keys, injects them into the container environment, and redacts secret values from logs and activity output.

For more detail, see [Variables and secrets](/docs/guides/variables-and-secrets).

## Auto Update

Auto update checks external image tags for changed content on the Deployment’s
platform. Changes only to other architectures or image attestations are ignored.

- `Disabled`: disable periodic checks; manual checks remain available.
- `Notify Only`: record update availability and emit an alert.
- `Auto Deploy`: pull the updated image and redeploy the container automatically.

`Notify Only` remains available in Community. `Auto Deploy` requires Team's
`Operational Guardrails` capability.

Citadel checks periodically. When an image update is found:

- Notify mode marks the deployment as having an update available.
- Auto Deploy mode recreates the container with the updated image.
- Failures emit deployment auto-deploy failure alerts when alert rules are configured.

Use **Check for updates** on a deployed external image to query its registry
immediately. This manual check records whether an image update is available but
does not pull, redeploy, or alert. It remains available when periodic update
behavior is disabled.

On the Deployments page, use **Updates available** to show only deployments
with a detected image update. This filter works with search, tags, and the
Platform filter. Deployments that have not been checked or whose last check
failed are not shown as having an available update.

Use `Notify Only` for production deployments until the image tag and rollback process are proven. Use explicit version tags for predictable updates.

Follow [Update applications](/docs/guides/application-updates) for the complete
check, deploy, verify, and recovery workflow.

## Resources

Resource profiles set CPU and memory limits for the container.

- `Automatic`: leave resource allocation to Docker.
- `X-Small`: 0.25 CPU, 256 MB RAM.
- `Small`: 0.5 CPU, 512 MB RAM.
- `Medium`: 0.5 CPU, 1 GB RAM.
- `Large`: 1 CPU, 2 GB RAM.
- `X-Large`: 2 CPU, 4 GB RAM.

Choose a limit when a workload should not be able to consume the host unexpectedly. Leave automatic for simple or trusted workloads where Docker defaults are acceptable.

## Lifecycle

Lifecycle controls container restart and shutdown behavior.

Restart policy:

- `No`: do not restart the container automatically.
- `Always`: restart whenever the container stops.
- `Unless Stopped`: restart unless a user manually stopped it.
- `On Failure`: restart only when the container exits with a non-zero code.

Stop signal:

- `SIGTERM`: default graceful shutdown signal.
- `SIGINT`: interrupt-style graceful shutdown.
- `SIGKILL`: force stop immediately.

Stop timeout is the number of seconds Docker waits for shutdown before forcefully terminating the container.

For services, `Unless Stopped` or `Always` is usually appropriate. For one-off or diagnostic containers, use `No`.

## Command And Labels

Command overrides the image default command.

Example:

```text
--config=/etc/app/config.yml
--log-level=info
```

Use labels for user-defined Docker metadata:

```text
com.example.owner=platform
com.example.service=api
```

Do not use labels reserved by Citadel.

## Deploy, Redeploy, And State Actions

Use **Deploy** for the first apply of a created deployment.

Use **Redeploy** when you want Citadel to recreate the container from the saved deployment definition. Redeploy is useful after changing image, ports, volumes, variables, lifecycle, resources, command, or labels.

Runtime actions:

- **Start** starts a stopped deployment.
- **Stop** stops a healthy deployment.
- **Pause** pauses a running deployment.
- **Resume** resumes a paused deployment.
- **Delete** removes the deployment and its managed runtime container.

If a platform is disconnected, deploy and runtime actions can fail because Citadel cannot reach the Docker host.

## Duplicate

The **Duplicate** menu on the detail page offers **Configuration only** and
**Configuration + data**. Configuration only opens a prefilled add form from
the saved configuration.

The duplicate flow:

1. opens the normal add form
2. copies editable configuration into a draft
3. shows warnings for platform-specific values such as host bind mounts
4. lets you edit the draft, then select **Review copy** before creating anything

The new Deployment stays undeployed. Automatic updates and build-triggered
redeployment are disabled; configure these on the new Deployment after reviewing
its targets. The source Deployment is unchanged.

**Configuration only** copies settings, **not data**. Named volumes, bind paths,
networks, ports and binding references can still point to the original storage
or external systems. Review these before Apply.

Runtime state, container IDs, history, generated credentials, resolved secret
values and access grants are not copied.

Deployment-scoped variables and secret references are copied when you save the duplicate. Global bindings continue to be inherited. You need permission to view and manage resource bindings to copy them.

**Configuration + data** uses a three-step wizard to name the copy, choose
current data or existing backups, and review the new storage. Copying current
data does not require a backup policy; confirm source consistency first. It restores
into fresh volumes before creating an undeployed Deployment. Existing backups in a
reachable S3-compatible repository can be restored on another Standalone Platform;
current-data copies stay on the source Platform. Bind mounts and anonymous mappings must be handled separately;
see [Copy a workload with data](/docs/resources/backups#copy-a-workload-with-data)
for requirements and recovery.

## Adopt An Existing Container

Use **Adopt as Deployment** from an unmanaged standalone container to create a
Deployment without restarting or recreating the current container. Citadel
opens the normal add form with a configuration draft and keeps the source
Platform fixed.

Adoption does not deploy anything. A later Redeploy can recreate the container
from the reviewed definition. Containers that belong to Docker Compose must be
imported as complete Stacks instead.

See [Adopt existing workloads](/docs/guides/adopting-existing-workloads) for eligibility, secret
handling, unsupported runtime settings, and first Redeploy guidance.

## Backups

Deployment backup policies cover explicitly named volume sources in the saved
configuration, such as `app-data:/app/data`, and supported bind mounts discovered
from the synchronized container on standalone Docker.

The backup shield at the far right of the main tab bar is available from every
tab. Hover, focus, or click it to see coverage for each volume and bind mount,
including the most recent successful backup when available.
Use **Configure backup** to open a prefilled policy form, or follow an existing
policy link to manage its schedule, retention, repository, and runs centrally.

Runtime-only anonymous volumes, such as those created from an image's `VOLUME`
instruction or a destination-only mount, are not included in deployment policies.
The card marks them **Not included in deployment backups**. Use the individual
volume's **Configure backup** link to create a separate Docker volume policy using
its generated name. That policy targets the exact volume, so it does not follow
a replacement anonymous volume after container recreation.

Supported bind mounts discovered from the synchronized deployment container
are included alongside declared named volumes. Review host paths in the source
preview. The policy creator and run-as identity need Platform write access for
host data. A direct **Host file or directory** policy can also protect a single
path. Tmpfs, nested mounts, system/runtime paths, and writable layers remain excluded.
The protected count covers listed storage targets, not all application state.

For backup setup and restore behavior, see [Backups](/docs/resources/backups).

Use **Backup Filter** at the right of the resource header to filter by backup status.
The resource table displays a **Backup** column. Hover over its shield to see
the same coverage card as on the resource page. Manual-only or disabled policies
show amber, even after a successful manual run. Green requires an active schedule,
a ready repository, and a successful available backup for all eligible storage.
See [Backup coverage](/docs/resources/backups#coverage-on-resource-pages) for all
status meanings and scheduling requirements.

## Diagnose a Failed Deployment

During **Deploy** or **Redeploy**, read the progress sheet for the operation's
output and error. If you have closed it, open the Deployment's **Activities** tab
or the main **Activities** page and select the failed **Deployment Applied** event.
The event details show the recorded configuration and failure message.

[![Deployment Applied activity details showing the configured image and a manifest unknown failure for a missing image tag](/screenshots/deployment-failure-details.png)](/screenshots/deployment-failure-details.png)

This demo failure refers to `nginx:demo-missing`. Compare the image reference in
**Applied configuration** with the failure message, correct the image tag under
**Config**, save, and deploy again. An activity named **Deployment Applied** can
have **Failure** status; its name alone does not indicate success.

Container logs describe the application's output after a container starts. A
failure while pulling an image can occur before there is a container to inspect.

The **Container** tab remains available when a Deployment is degraded or stopped.
Use **Logs** and **Inspect** while its container still exists and the Platform is
reachable. Each tool requires its own permission. **Terminal** additionally
requires a running container and no Deployment operation in progress.

If the Platform is offline, live tools are unavailable. Previously loaded
container details are marked as last known; retained statistics and **Activities**
can still help explain the failure. Deleted containers may no longer have logs or
inspection data. A Deployment that has never created a container shows **Not
deployed** instead.

## When To Use Deployments

Use deployments for:

- one-container services
- utility containers
- simple reverse-proxy targets
- workloads that do not need Docker Compose

Use stacks for:

- multiple containers deployed together
- shared Compose networks and volumes
- Compose overrides or advanced Compose syntax
- Git-backed deployment workflows
