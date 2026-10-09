---
title: "Images"
description: "Pull and inspect Docker images on a Platform and remove images you no longer need."
---

Open a **Platform**, then **Images** to view images available on its Docker host.
A [Registry](/docs/resources/registries) supplies images; this page shows the
copies stored on the host.

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

Use **Inspect** to review image metadata, layers, and associated containers.
Check which containers use an image before removing it. Tags can move to a new
image over time; the image ID identifies the local image you are inspecting.

On Swarm, pay attention to the Node associated with an image. An image present
on one Node is not necessarily available on every Node. Node-specific inventory
supports inspection; it does not offer arbitrary image deletion on that Node.

## Remove an image

Select the image, choose **Delete**, and review the confirmation. Keep images
needed by local-image Deployments or an offline recovery procedure. Removing a
local image does not remove it from its Registry.

If a pull fails, read the operation error and check the image name, tag, Registry
credentials, and network access from the target Docker host. See
[Registries](/docs/resources/registries) for authentication
and private Registry setup.
