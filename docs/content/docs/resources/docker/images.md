---
title: "Images"
description: "Pull and inspect Docker images on a Platform and remove images you no longer need."
---

Open a **Platform**, then **Images** to view images available on its Docker host.
A [Registry](/docs/resources/registries) supplies images; this page shows the
copies stored on the host.

The compact table groups the image name and primary tag in the **Image** column,
keeping the usage/processing indicator beside the name.
Select **+N** to see additional tags. **Update status**, **Used by**, and **Size**
remain visible; Swarm platforms also show **Node**. Use **Columns** beside
**Pull Image** in the page toolbar to show the
optional **ID**, **Registry**, and **Created** columns. The copyable image ID is
hidden by default. These choices are remembered in
your browser.

## Pull an image

You need an online Platform, permission to pull images, and access to the
selected Registry.

1. Select **Pull Image**.
2. Choose the **Registry** and enter the **Image**, for example `nginx:alpine`
   with **Docker Hub**.
3. Select **Pull image** and follow the operation's progress.
4. When it finishes, find the image in the Platform's image list.

Pulling downloads an image; it does not start a container or update an existing
application. Use a [Deployment](/docs/resources/deployments#image-source) to run
it, or the owning workload's update controls to apply a newer version.

## Inspect an image

Use **Inspect** to review image metadata, layers, update status, and associated workloads.
Check which containers use an image before removing it. Tags can move to a new
image over time; the image ID identifies the local image you are inspecting.

On Swarm, pay attention to the Node associated with an image. An image present
on one Node is not necessarily available on every Node. Node-specific inventory
supports inspection; it does not offer arbitrary image deletion on that Node.

## Check for updates

Use **Updates available** beside **Pull Image** to show images with at least one
fresh detected tag update. It works alongside search and excludes stale-only,
failed, and unchecked results. Select it again to show all images.

The **Update status** column uses a compact status icon and label. Hover, focus,
or tap it to see cached results for each tag, check times, and errors. The check
dialog and status card use the same result explanation. Expand **Technical details**
to inspect configuration and manifest digests; these are hidden by default.
The same indicator appears as an icon in **Image details**. Select an image
and choose **Check for updates**, or use the action in image details. Choose the
exact tag and an authorized matching Registry. Checks fetch metadata only; they
do not download layers or redeploy workloads. Opening the table does not contact
registries. A manual check requests a fresh observation.

If the image has no associated Registry, a successful manual check remembers
your selection when you have the Platform's **Pull** permission. Future checks
and **Pull latest** can reuse it. Failed or inconclusive checks do not save the
selection, and checking with another Registry does not replace an existing
association. Read-only users can check for updates without changing the image's
Registry.

Citadel automatically checks tagged images on online standalone hosts every two
hours. A background scan runs every minute to pick up newly inventoried images
and tags, including images pulled by Deployments or Stacks. Large inventories
and unavailable registries can delay a sweep. Checks use the associated Registry,
a previously selected matching Registry, or a single matching configured Registry;
with none configured they try anonymously. Ambiguous Registry choices require a
manual check. Untagged, digest-pinned, and Swarm node-local images are excluded.

Results are saved before the Images list and image details are notified. When a
pull moves a tag to a new image ID, the new image gets its own check; the previous
image does not inherit the new result. This follows observed Docker inventory,
regardless of which workload initiated the pull. Scheduled checks only inspect
metadata; they never pull an image or redeploy a workload.

A successful manual pull invalidates cached comparisons for the observed image,
even when Docker keeps the same image ID. Citadel refreshes an existing comparison
for the pulled tag before the progress stream finishes. If that check cannot run,
the background scan can retry it; a failed metadata check does not undo or fail
the completed pull.

A tag on a locally built image is not proof that it exists in a Registry. For
example, `citadel-agent:local` resolves to `docker.io/library/citadel-agent:local`.
An unpublished or private reference can return an access-denied or not-found
result. Publish the image under its intended repository/tag and select matching
Registry credentials before checking it. A tag named `local` is not automatically
excluded: published tags with that name can still be checked.

**Up to date** means the selected image matches the registry tag for this host.
**Update available** means the tag has different image content for this host,
not necessarily a newer software version. Changes only for other architectures
do not count as an update. If required registry metadata is unavailable, Citadel
cannot determine whether an update is available.
For multiple tags, open the status summary to see each result. Failed checks retain
the previous successful observation separately. Results survive Core restarts and become stale after one day without a successful
check; stale inventory cannot appear currently up to date.

Choose **Pull latest** to start pulling that exact tag on the current host and
open the progress sheet directly from the Images list or image details. Moving
between these views keeps the same progress sheet and pull request running.
Images with multiple tags ask which tag to pull.
For example, pulling `postgres:17` keeps `17`; it does not choose `latest` or
another major version.

**Pulling a newer image alone does not update workloads that use it.** Existing
containers keep their current image. Deployments, Stacks, and Swarm Services
continue to follow their own update settings: **Notify Only** reports available
updates, while an enabled automatic deployment policy can apply them separately.
Otherwise, update or redeploy the owning workload yourself.

An image can show **Up to date** while a workload still shows **Update available**:
the host has downloaded the new image, but the workload is still using the old
one. Pulling does not clear the workload's update status. See
[Update applications](/docs/guides/application-updates) for checking, applying,
and automating workload updates.

Citadel uses the image's associated Registry when it matches and you can access
it, otherwise a single authorized matching Registry. With no matching Registry,
it tries anonymously; a configured Registry is not required for public images.
Multiple matching Registries require a choice in the progress sheet. If a pull
fails, the sheet lets you select Registry credentials and retry, or open Registry
settings to configure them. Retries are explicit; Citadel does not retry a failed
pull automatically.

Local, Direct, and Edge standalone hosts support metadata checks. Older Agents
without the comparison metadata show **Cannot determine**. Registry metadata
reads require verified HTTPS; registries that need insecure HTTP, unsupported
authentication redirects, or ambiguous platform metadata cannot be compared.
Swarm-node image inspection and usage remain available, but node-local checks and
contextual pulls are unavailable under the current Agent restrictions.

## Find image references

The **Used by** column summarizes known container, Deployment, Stack, and managed
Swarm Service references in workloads you can read. The list response includes
these summaries; opening the table does not issue a separate request per image.
**None found** means no known references were found. When coverage is partial,
hover over or focus the usage link to see the coverage explanation.
In image details, **Workloads using this image** combines the existing container
table with managed owners and saved Local image, tag, and digest references.
Workload links use Citadel's recorded ownership. On Swarm, observed containers are scoped to the selected Node.
Container states refresh when Citadel receives lifecycle updates. Deployments,
Stacks, and Swarm Services each have a separate table using the Containers table
layout. Workload tables and headings are shown only when they have matching
entries on the current page. Each name has a status indicator from the recorded
workload status (current release status for Stacks and health for Swarm Services),
with reference and observation details in the remaining columns. Use the reload
icon at the right of the **Workloads using this image** heading to refresh
references.

Use **More references** in image details to continue through large inventories.
A page without matches can still offer more references to check;
it does not mean the remaining inventory has no references.

Saved tag references describe configuration; they do not prove which image a
running container uses. Coverage is partial for Git, variable-dependent Compose,
build references, and historical releases. Digest matching requires repository
digests in inventory; the standalone inventory does not yet persist them.
No known references is not a guarantee that deleting an image is safe.

## Remove an image

Select the image, choose **Delete**, and type the requested confirmation.
For a single image, enter its name (or its ID if no name is available).
For bulk deletion, the dialog lists the selected images and asks you to enter
**Delete**. Click the bold confirmation text to copy it.
Delete is disabled for images used by containers. Opening the confirmation
dialog does not make an additional usage-check request.
Keep images needed by an offline recovery procedure. Removing a local image does
not remove it from its Registry.

Deletion from the list and image details is non-forced and does not prune untagged
parent images. Known container references, including stopped containers, block
the action. Refresh stale inventory before deleting. Docker can still reject
removal if usage changes or the image has multiple tags; Citadel does not retry
with force or automatically remove all tags.

The backend checks live container usage when deletion is submitted.
Saved Local image Deployment dependencies also block non-forced deletion, even
when you cannot read the owning Deployment; hidden workload names are not exposed.
Configuration saves and deletion claims coordinate on the same image so a new
Local image dependency cannot be saved while deletion is processing.

Deletion can still be rejected if image usage or permissions have changed.
Resolve the reported error before retrying. Pulling a newer image moves its tags in the
inventory while preserving the old local image and its saved dependencies.

If a pull fails, read the operation error and check the image name, tag, Registry
credentials, and network access from the target Docker host. See
[Registries](/docs/resources/registries) for authentication
and private Registry setup.
