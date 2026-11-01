---
title: "Docker Standalone"
description: "Connect one Docker host and manage its containers and Compose applications."
---

A Docker Standalone Platform connects to one Docker engine. Use it for
[Deployments](/resources/deployments) and [Compose Stacks](/resources/stacks).
For cluster scheduling, use a [Docker Swarm Platform](/resources/platforms/docker-swarm).

## Add a Platform

1. Open **Platforms** and select **Add Platform**.
2. Choose **Docker Standalone**.
3. Choose **Local**, **Agent**, or **Edge Agent** for the connection.
4. Enter the connection settings and save the Platform.
5. For Edge Agent connections, run the generated enrollment command on the Docker host.
6. Open the Platform when it comes online to see its Docker resources.

For a Local connection, Core needs access to the host's Docker socket. For a
remote connection, follow [Agent setup](/operations/agent) or
[Edge Agent setup](/operations/edge-agent).

## Explore Docker resources

| Page | Common tasks |
| --- | --- |
| [Containers](/resources/docker/containers) | Read logs, inspect configuration, open a terminal, and manage state |
| [Images](/resources/docker/images) | Pull an image and inspect its layers and container usage |
| [Networks](/resources/docker/networks) | Create networks and inspect attached containers |
| [Volumes](/resources/docker/volumes) | Create storage, browse files, and download contents |

Your permissions and the Platform connection determine which actions are available.

Existing Docker resources can be visible before they are managed as Citadel
Deployments or Stacks. Use [adoption](/guides/adopting-existing-workloads)
when you want Citadel to manage an existing application's configuration.

## Deploy applications

- Use a [Deployment](/resources/deployments) for one container.
- Use a [Web Editor Stack](/resources/stacks/web-editor) for Compose content stored in Citadel.
- Use a [Git Stack](/resources/stacks/git) for Compose files stored in a repository.

For metrics and connection failures, see [Platform monitoring](/operations/platform-monitoring)
and [troubleshooting](/operations/troubleshooting).
