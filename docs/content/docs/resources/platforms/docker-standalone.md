---
title: "Docker Standalone"
description: "Connect one Docker host and manage its containers and Compose applications."
---

A Docker Standalone Platform connects to one Docker engine. Use it for
[Deployments](/docs/resources/deployments) and [Compose Stacks](/docs/resources/stacks).
For cluster scheduling, use a [Docker Swarm Platform](/docs/resources/platforms/docker-swarm).

## Add a Platform

1. Open **Platforms** and select **Add Platform**.
2. Choose **Docker Standalone**.
3. Choose **Local**, **Agent**, or **Edge Agent** for the connection.
4. Enter the connection settings and save the Platform.
5. For Edge Agent connections, run the generated enrollment command on the Docker host.
6. Open the Platform when it comes online to see its Docker resources.

For a Local connection, Core needs access to the host's Docker socket. For a
remote connection, follow [Agent setup](/docs/operations/agent) or
[Edge Agent setup](/docs/operations/edge-agent).

## Explore Docker resources

| Page | Common tasks |
| --- | --- |
| [Containers](/docs/resources/docker/containers) | Read logs, inspect configuration, open a terminal, and manage state |
| [Images](/docs/resources/docker/images) | Pull an image and inspect its layers and container usage |
| [Networks](/docs/resources/docker/networks) | Create networks and inspect attached containers |
| [Volumes](/docs/resources/docker/volumes) | Create storage, browse files, and download contents |

Your permissions and the Platform connection determine which actions are available.

Existing Docker resources can be visible before they are managed as Citadel
Deployments or Stacks. Use [adoption](/docs/guides/adopting-existing-workloads)
when you want Citadel to manage an existing application's configuration.

## Deploy applications

- Use a [Deployment](/docs/resources/deployments) for one container.
- Use a [Web Editor Stack](/docs/resources/stacks/web-editor) for Compose content stored in Citadel.
- Use a [Git Stack](/docs/resources/stacks/git) for Compose files stored in a repository.

For metrics and connection failures, see [Platform monitoring](/docs/operations/platform-monitoring)
and [troubleshooting](/docs/operations/troubleshooting).
