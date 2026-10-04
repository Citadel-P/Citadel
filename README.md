<img src="docs/public/logo.svg" alt="Citadel logo" width="64" height="64">

# Citadel

**Deploy and operate your Docker applications from one place.**

Citadel is a self-hosted Docker management platform for developers and teams. Build images from Git, deploy containers and Compose stacks, and manage Docker Standalone and Swarm across multiple hosts through one web interface.

[Documentation](docs/content/docs/index.mdx) · [API Preview](docs/content/docs/reference/api.mdx) · [OpenAPI schema](schema/public-v1.json) · [Report an issue](https://github.com/Citadel-P/Citadel/issues)

![Citadel Compose stack details with running services, deployment controls, and logs](docs/public/screenshots/stacks.png)

<details>
<summary>Platforms, containers, and monitoring</summary>

![Citadel Platforms page with an online standalone Docker host and an online Swarm host](docs/public/screenshots/platforms.png)

![Citadel Docker platform with five running containers and live CPU and memory usage](docs/public/screenshots/platform-containers.png)

</details>

<details>
<summary>Swarm services and replica configuration</summary>

![Citadel Swarm overview with three healthy services and their running replica counts](docs/public/screenshots/swarm-services.png)

![Citadel Swarm service configuration showing replica settings and deployment controls](docs/public/screenshots/swarm-service-config.png)

</details>

<details>
<summary>Dark mode</summary>

![Citadel dark-mode Platforms page showing Docker and Swarm hosts](docs/public/screenshots/platforms-dark.png)

</details>

## What you can do

- **Deploy applications** — manage containers and Compose stacks, deploy from Git, roll back releases, and check configuration drift.
- **Connect your hosts** — manage local Docker, remote Agents, and outbound Edge Agents; inspect resources, logs, and terminals.
- **Operate Swarm** — manage nodes, services, tasks, configs, and secrets.
- **Build and automate** — build images from Git, publish to registries, and run TypeScript actions and webhooks.
- **Monitor and recover** — track resource usage, receive alerts, and back up and restore the control plane and Docker volumes.
- **Control access** — manage users, teams, roles, and resource permissions, with MFA, OIDC sign-in, and activity history.

## Try Citadel

The steps below build the current Rust code from `main`. For published images, follow the [installation guide](docs/content/docs/getting-started/install.mdx).

With Git, Docker, and Docker Compose 2.30 or newer on a Linux Docker host:

```bash
git clone --branch main --single-branch https://github.com/Citadel-P/Citadel.git
cd Citadel/deploy
cp .env.example .env
```

Set a strong, unique `PG_PASSWORD` in `.env`, then start Citadel and PostgreSQL:

```bash
docker compose up -d --build
```

Open [http://localhost:18000](http://localhost:18000), create your administrator account, and add a **Local** platform to manage Docker on this host. There are no default credentials.

The Compose setup grants Citadel administrative access to the local Docker host. For network access, follow the [configuration](docs/content/docs/getting-started/configuration.md) and [TLS guide](docs/content/docs/operations/tls-and-secure-agent-transport.md).

## Documentation and API

Start with the [quick start](docs/content/docs/getting-started/quick-start.mdx), [task guides](docs/content/docs/guides/index.mdx) and [architecture overview](docs/content/docs/overview/architecture.mdx).

For integrations, use the [public OpenAPI schema](schema/public-v1.json) and [API Preview guide](docs/content/docs/reference/api.mdx). The full schema used by the web UI is internal.

On your installation, set `EnableSwagger=true` in `.env` and run `docker compose up -d server` to enable `/swagger/` and `/openapi/public/v1.json`.

## Editions and support

Community runs without an installed product license. See the [edition comparison](docs/content/docs/overview/licensing.md) for included and paid capabilities.

For source development, see the [development guide](docs/DEVELOPMENT.md). Report bugs and request features through [GitHub Issues](https://github.com/Citadel-P/Citadel/issues).
