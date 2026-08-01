# Adopting Existing Docker Workloads

Citadel can bring containers that are already running on a Docker Platform
under management without stopping or recreating them.

Use:

- **Adopt as Deployment** for one standalone container
- **Import Compose project as Stack** for containers started by Docker Compose

Adoption is available in every Citadel edition.

## What Adoption Changes

Adoption creates a Citadel resource and links it to the existing Docker
container or Compose project.

Adoption does not:

- restart or stop containers
- recreate containers
- run Docker Compose
- pull images
- rename containers
- change Docker labels
- change networks or volumes
- apply the generated configuration

The existing workload keeps running while you review and save its Citadel
configuration.

## Adopt A Standalone Container

On the **Containers** page:

1. Open the actions for an unmanaged standalone container.
2. Select **Adopt as Deployment**.
3. Review the generated Deployment configuration.
4. Resolve any warnings or required values.
5. Select **Adopt Container** and confirm.

Citadel opens the normal add Deployment page instead of placing the complete
form in a dialog. The Platform is fixed because the running container already
belongs to that Docker host.

After adoption, Citadel opens the Deployment page and displays the existing
container and its current state. You do not need to deploy it again merely to
complete adoption.

## Review The Generated Configuration

Citadel reads the current Docker configuration and pre-fills the settings it
can manage, including:

- image
- published ports
- named volumes and bind mounts
- networks
- command
- environment variables
- restart and stop behavior
- CPU and memory limits
- user labels

Review the draft before adopting it. Docker supports settings that may not yet
exist in Citadel's Deployment form. Citadel blocks adoption when an unsupported
setting would make a later Redeploy materially change the workload without a
clear representation.

Host bind mounts are Platform-specific. Confirm that every source path is
intentional and remains available on the Docker host.

If Docker has pruned the container's original image, Citadel leaves the
**Local Image** field empty and asks you to select a synchronized replacement.
You can select a synchronized local image or an external image from the same
repository as the container's original image. Adoption does not pull an external
image or restart the container, but future Apply operations use the replacement.
Citadel inspects local replacements before adoption and blocks incompatible
process defaults. A build image cannot replace a missing original image during
adoption because selecting a build project does not guarantee that an inspectable
artifact exists on the target Platform.

## Environment Values And Secrets

Docker inspection can contain passwords, tokens, and other sensitive
environment values. Citadel does not reveal those values in the adoption
draft.

For each masked value, choose one of these approaches before adoption:

- enable **Import detected values as Citadel secrets** to create encrypted,
  deployment-scoped secret bindings without sending the values to the browser
- reference an existing Citadel variable or secret
- enter an intentional literal value
- remove the variable after confirming that the application does not need it

Prefer Citadel secrets for credentials. A literal environment value is stored
in the Deployment definition.

Imported values are read directly from the container during the confirmed
adoption request. Citadel encrypts each value before storing it and keeps only
an explicit `NAME=${NAME}` binding reference in the Deployment definition.
This option is enabled by default when every detected sensitive value can be
imported.

Citadel never stores the displayed masking value as the real container
configuration.

## Import A Compose Project

A container with Docker Compose project membership cannot be adopted as a
standalone Deployment. Import the complete project as one Stack so its
services, networks, and volumes remain a coherent unit.

From a Compose container or unmanaged project group:

1. Select **Import Compose project as Stack**.
2. Choose **Web Editor** or **Git** as the authoritative Compose source.
3. Supply the Compose file or select the repository and Compose paths.
4. Review the detected and defined services.
5. Confirm **Import Compose Project**.

Citadel does not reconstruct Compose YAML from running containers. Docker does
not retain enough information to recover variables, profiles, anchors, build
configuration, comments, and other source details reliably.

The imported Stack keeps the detected Docker Compose project name. This allows
a later Apply to target the existing project instead of creating a second set
of containers.

## Safe Initial Settings

Import drafts default to no automatic runtime changes:

- image update behavior is disabled
- Stack drift monitoring and automatic correction are disabled
- Stack webhooks are disabled
- imported Stacks do not destroy the project before the first Apply
- Citadel does not Apply automatically

You can enable update and webhook behavior while reviewing the import or later.
Stack drift behavior can be enabled after import.

## The First Redeploy Or Apply

Adoption and Apply are separate operations.

The first later **Redeploy** of an adopted Deployment may recreate its
container from the saved Deployment definition.

The first later **Apply** of an imported Stack may update or recreate
containers so they match the authoritative Compose source.

Before the first Redeploy or Apply:

1. Review images, ports, mounts, networks, variables, and lifecycle settings.
2. Confirm that required Citadel variables and secrets resolve successfully.
3. Back up important named volumes.
4. Schedule downtime when the workload cannot tolerate container recreation.

Deleting the resulting Deployment or Stack follows normal Citadel behavior and
can delete its managed runtime containers.

## Eligibility

Citadel offers adoption only when:

- the container is not already managed
- the container is not a Citadel System container
- the Docker Platform is connected
- the current user can inspect the Platform and create the target resource
- the runtime configuration can be represented safely

Citadel Core, PostgreSQL, Agent, and Edge Agent containers are System
containers and cannot be adopted.

Compose projects whose containers consistently reference the same Citadel
Stack ID can be recovered when that Stack no longer exists in the current
database. Citadel rejects the import when the referenced Stack still exists,
the labels are malformed, the project mixes labeled and unlabeled containers,
or its containers reference different Stack IDs.

Standalone containers with Citadel ownership labels remain ownership
conflicts and cannot be adopted as new Deployments.

## External Container Recreation

Citadel links an adopted standalone Deployment to the exact Docker container
ID that was reviewed.

If another tool deletes and recreates that container, Docker assigns a new ID.
Citadel cannot safely assume that the new container is the same workload. The
replacement appears unmanaged and must be reviewed again.

Compose Stacks receive current Citadel ownership labels through the normal
Stack Apply process after import or orphan recovery.

## Permissions

Standalone adoption requires:

- Deployment Write permission
- Platform Read permission
- Platform Inspect permission
- access to referenced images, registries, tags, variables, and secrets

Compose import requires:

- Stack Write permission
- Platform Read permission
- Platform Inspect permission
- access to the selected Git Repository and other referenced resources

Apply permission is not required merely to adopt or import because the
operation does not change Docker runtime resources.

## Troubleshooting

**The adoption action is not visible**

The container may already be managed, belong to Compose, be a System
container, or require permissions you do not have.

**The draft says the container changed**

The container was modified after Citadel generated the draft. Reload the
draft, review the current runtime configuration, and confirm again.

**The Platform is unavailable**

Citadel must inspect the current runtime before changing ownership. Reconnect
the local, Agent, or Edge Agent Platform and retry.

**A runtime option blocks adoption**

Citadel cannot yet preserve that non-default option in a Deployment. Keep the
container unmanaged or first change the workload through Docker after
understanding the operational impact.

**A Compose source does not match**

Verify the Compose project name and service names. Select the source that
actually defines the running project; do not use a similar project with a
different name.

For normal Deployment behavior, see `docs/user/deployments.md`.

For Web Editor Stacks, see `docs/user/web-editor-stacks.md`.

For Git-backed Stacks, see `docs/user/git-stacks.md`.
