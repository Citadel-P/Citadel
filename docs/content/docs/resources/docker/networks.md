---
title: "Networks"
description: "Create Docker networks, connect managed workloads, and inspect network usage."
---

Open a **Platform**, then **Networks** to inspect or create Docker networks.
Containers use networks to communicate; publishing a container port separately
makes that port reachable through the Docker host.

## Browse networks

Search by network, driver, subnet, Node, or known workload name. Use **Driver**,
**Scope**, and **Usage** filters to narrow the list, and **Columns** to choose
visible details. Turn off **Show system networks** to hide Docker's built-in networks.

Select a network to see its addresses, connected containers, and configured
workloads. **Configured workloads** are saved references, not proof of an active
connection. Unresolved references, including some Git-backed external networks,
may be missing.

**In use** includes Docker-reported usage even when attachment details are
unavailable. **Has visible attachments** requires visible attachment evidence;
**Unknown** includes stale observations. **No reported use** does not prove a
network is safe to delete. Swarm results can cover only part of the cluster.

## Explore connections

Select a network, or choose **View connections** on a Deployment, Stack, or
Swarm Service. Use **Maximize network sheet** for a larger view. Platform Read
access is required, and only workloads you can read are shown. Full container
inspection requires **Inspect** permission separately.

![Storefront stack connected to frontend and backend networks, with network details alongside the map](/screenshots/network-map.png)

- Select a resource for details, or expand a Stack to see its containers.
- Use **Map selection** to search or navigate with the keyboard.
- Solid lines show reported attachments. Dashed lines show configured use or
  unconfirmed attachments; they do not prove an active connection.
- **Map options** controls configured references and layout. Use zoom and
  **Fit view** to navigate.
- **Explore connections** on a container shows its networks, addresses, aliases,
  and published ports. Host, none, and shared-namespace modes may have no graph.

The diagram updates while open. It displays up to 50 relationships, and workload
search lists up to 500 accessible containers. Partial or stale results are marked;
open a container directly if it is absent from the search selection.

The map shows Docker relationships, not a connectivity test. Configured networks
can remain visible for stopped containers or Services scaled to zero. It does not
verify application reachability or VPN status.

## Create a network

You need an online Platform and permission to create networks.

1. Select **Add Network** and enter a name, such as `app-network`.
2. For Docker Standalone, keep **Bridge** for a typical application network.
3. Leave address allocation at its defaults unless you need a particular subnet
   or gateway. If you specify a subnet, avoid overlap with existing networks.
4. Select **Save** and check that the network appears in the list.

On Swarm, choose **Swarm** scope and **Overlay** for a cluster network. **Local manager**
scope creates a network only on the connected manager. Enable **Attachable**
when standalone containers also need to join an overlay network.

## Use the network

For a Deployment, select the network in its configuration, save, and deploy or
redeploy. Creating the network alone does not attach an existing container.

For a Stack, Compose can create its own networks. To use the network you created
above, declare it as external and attach the service:

```yaml
services:
  web:
    image: nginx:alpine
    networks:
      - app-network

networks:
  app-network:
    external: true
```

This example joins a network without publishing a host port. Add a port mapping
or connect your reverse proxy when the application needs access from outside it.

## Inspect or remove a network

Use **Inspect** to review the driver, address allocation, and attached containers.
If applications cannot communicate, check that they share the intended network
and use the application's container port.

**Delete** is unavailable for system networks, networks in use, visible saved
references or service memberships, and Node-specific network inventory. Remove workload references and redeploy affected workloads
before deleting an unused network. For Swarm, also check Service attachments.

Citadel's network form creates networks; it does not edit an existing network's
configuration. Create a replacement when its settings need to change.
