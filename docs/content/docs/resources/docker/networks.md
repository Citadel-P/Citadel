---
title: "Networks"
description: "Create Docker networks, connect managed workloads, and inspect network usage."
---

Open a **Platform**, then **Networks** to inspect or create Docker networks.
Containers use networks to communicate; publishing a container port separately
makes that port reachable through the Docker host.

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

**Delete** is unavailable for system networks, networks in use, and Node-specific
network inventory. Remove workload references and redeploy affected workloads
before deleting an unused network. For Swarm, also check Service attachments.

Citadel's network form creates networks; it does not edit an existing network's
configuration. Create a replacement when its settings need to change.
