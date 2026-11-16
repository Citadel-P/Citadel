---
title: "Volumes"
description: "Create Docker volumes, browse and download their contents, and protect application data."
---

Open a **Platform**, then **Volumes** to see Docker-managed storage. Named volumes
persist independently of a container. Bind mounts instead use paths on the host;
they are not available in the volume browser.

A shield beside the name identifies Citadel system storage. Use **Backup Filter**
to find volumes by backup status.

## Create and attach a volume

1. On an online Platform, select **Add Volume**.
2. Enter a name such as `app-data`. Keep the **local** driver unless your storage
   setup requires another installed Docker volume driver.
3. Select **Save** and check that the volume appears in the list.
4. Add a mount to your Deployment, such as `app-data:/data`, then save and deploy
   or redeploy it. Use the data path required by your application.

A Stack can declare and create its own named volumes in Compose. To attach an
existing one, use an external volume with the matching name. Creating a volume
does not attach it to containers automatically.

On Swarm, a volume created here belongs to the connected manager. Local volumes
with the same name on different Nodes can contain different data. Check the
Node before browsing, backing up, or restoring one.

## Browse and download files

You need volume-content access and a reachable Docker host. For another Swarm
Node, [Cluster node coverage](/resources/platforms/docker-swarm#understand-cluster-node-coverage)
must provide a usable connection to that Node.

1. Select the volume and choose **Browse**.
2. Open directories and use the breadcrumbs to navigate.
3. Hover over a file or folder to reveal its download action. Files download
   individually; folders download as `.tar` archives.

![Volume browser with expanded image and document folders, file sizes, and a download action](/screenshots/volume-browser.png)

Browsing is read-only: it does not upload, edit, rename, or delete files. You can
download a selected file or subdirectory, but not the volume root. Symbolic links
cannot be followed or downloaded as files.

The browser reads current contents. Files may change while an application writes
to the volume; a download is not an application-consistent backup. For recoverable
snapshots and retention, configure a [backup policy](/resources/backups).

## Protected storage

Browsing and downloads are disabled for Citadel's system storage, including for
administrators. The browser supports **local** volumes without driver options;
custom storage drivers and options are not supported. You can still inspect
volume metadata.

If Citadel cannot verify that a volume is safe to browse, file access stays
disabled. Grant **Browse** and **Download** only to trusted users: application
volumes can contain credentials and other sensitive data.

For custom installations, label Citadel data volumes with `com.citadel.system: "true"`
when provisioning them so they remain identified as system storage after their
containers are removed. Changing Compose does not relabel existing volumes;
never delete a data volume just to change its labels.

## Backup status

The **Backup** column shows coverage from policies you can access:

- **Protected** requires an enabled schedule and an available successful backup.
- A policy without a schedule needs attention.
- **Unavailable** means coverage could not be determined.

Hover or focus the icon in the **Backup** column to see policies, schedule warnings,
and the last successful backup. Use **Configure backup** when available, or open
an existing policy. See [backup coverage](/resources/backups) for details.

## Inspect and remove a volume

Use **Inspect** to review metadata and attached containers. Before **Delete**,
confirm that the data is no longer needed or that you have a usable backup.
Deleting a volume removes its stored data.

Deletion is unavailable while the volume is in use or when viewing Node-specific
inventory. A stopped container can still reference a volume; inspect its
attachments before treating it as unused.

## If browsing fails

| Symptom | What to check |
| --- | --- |
| **Browse** is disabled or access is denied | Check the storage limits above, your volume-content permissions, Platform status, and the selected Swarm Node's coverage |
| The browser cannot start | The target Docker host must be able to run Citadel's volume helper and obtain its image; inspect the returned error |
| A file has disappeared | Refresh the directory; the application may have moved or deleted it |
| A path cannot be downloaded | Select a regular file or subdirectory, rather than the volume root or a symbolic link |
