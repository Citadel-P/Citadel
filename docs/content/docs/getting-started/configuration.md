---
title: "Docker Compose configuration"
description: "Change Citadel settings without losing your installation data."
---

Citadel's installation settings live in `.env` beside `docker-compose.yml`
in your installation directory. Application settings,
such as Platforms and Deployments, are managed in the browser.

## Prepare the environment

The [installation guide](/getting-started/install) downloads the environment
template as `.env`. Open that file in a text editor to change settings.

Enter the database password without surrounding quotes. A long, randomly generated
password containing letters and numbers avoids environment-file parsing problems.

Do this only for a new installation. Keep your existing `.env` when upgrading.
The example is a **local HTTP setup**, not a ready-made public installation.

## Settings most installations need

| Setting | Example default | When to change it |
| --- | --- | --- |
| `PG_PASSWORD` | Empty | Always set a long, unique database password before starting |
| `CITADEL_HTTP_PORT` | `18000` | Change if this host port is already in use |

Compose supplies the remaining defaults. The local browser URL follows
`CITADEL_HTTP_PORT` unless you explicitly set `CITADEL_PUBLIC_URL`.

## Advanced installation settings

Add these to `.env` only when needed.

| Setting | Default | When to change it |
| --- | --- | --- |
| `CITADEL_IMAGE` | Unset → `ghcr.io/citadel-p/citadel:latest` | Optional: pin a specific version or use another registry |
| `CITADEL_BIND_ADDRESS` | `127.0.0.1` | Change only after configuring secure network access |
| `COMPOSE_PROJECT_NAME` | `citadel` | Choose a separate project for a separate installation; keep it unchanged for upgrades |
| `CITADEL_EDGE_PORT` | `18001` | Host port used by the optional Edge overlay |
| `CITADEL_TRANSPORT_MODE` | `Disabled` | Choose `ReverseProxy` or `Direct` for HTTPS network access |
| `CITADEL_PUBLIC_URL` | `http://localhost:18000` | Set to the browser address users will open |
| `CITADEL_EDGE_PUBLIC_GRPC_URL` | `http://localhost:18001` | Set an HTTPS URL whenever Core uses HTTPS, even without Edge Agents; see below |
| `CITADEL_HTTP_ALLOWED_HOSTS` | Local hostnames | Include the hostname of your shared installation |

Compose supplies PostgreSQL defaults of `PG_HOST=pg_db`, `PG_PORT=5432`,
`PG_USER=citadel`, and `PG_DATABASE=citadel`. These match the Compose database
service. Do not change them independently on an existing installation.

## Edge Agent access

The basic installation publishes only the browser/API port. Managing the local
Docker engine does not need an Edge Agent port.

When Core uses HTTPS through a proxy or serves TLS itself, it also requires an
HTTPS `CITADEL_EDGE_PUBLIC_GRPC_URL`. If you do not use Edge Agents, set it to the same HTTPS URL
as `CITADEL_PUBLIC_URL` and leave Edge port publishing disabled. Include the URL's
hostname in `CITADEL_HTTP_ALLOWED_HOSTS`. Before connecting Edge Agents, configure
a reachable gRPC endpoint and update this URL to match it.

If you use Edge Agents with a proxy installed on the host, or with direct Core
TLS, download the optional overlay into your installation directory:

```bash
curl -fSL https://raw.githubusercontent.com/Citadel-P/Citadel/main/deploy/install/compose.edge.yml -o compose.edge.yml
```

Include `-f docker-compose.yml -f compose.edge.yml` in your Compose commands,
along with any TLS overlay. It publishes the Edge listener on loopback port
`18001` by default; `CITADEL_EDGE_PORT` changes that host port. Configure
[TLS and secure Agent transport](/operations/tls-and-secure-agent-transport)
before allowing remote connections.

Containerized proxies on the same private Docker network connect to
`server:8001` directly and do not need this overlay. See the
[Caddy](/operations/reverse-proxy-caddy) or
[Nginx](/operations/reverse-proxy-nginx) guide.

## Transport mode

- **Disabled**: HTTP for local evaluation on loopback or an isolated test network.
- **ReverseProxy**: an HTTPS proxy sits in front of Citadel. Configure the immediate trusted proxy addresses and public URLs.
- **Direct**: Citadel serves TLS. Configure certificate and key paths, then include `compose.direct-tls.yml`.

The complete instructions are in
[TLS and secure Agent transport](/operations/tls-and-secure-agent-transport).
Browser traffic and Edge Agent traffic use separate listeners. Their container
ports (`8000` and `8001`) differ from the default host ports (`18000` and `18001`).

## Optional settings

| Setting | Default | Purpose |
| --- | --- | --- |
| `CITADEL_PASSWORDS_MINIMUM_LENGTH` | `15` | Minimum length for new or changed passwords; accepts 8–128 characters, with a fixed maximum password length of 128 |
| `CITADEL_MFA_POLICY` | `Optional` | Minimum MFA policy; administrators can choose a stronger policy in Administration → Instance |
| `CITADEL_UPDATE_CHECKS_ENABLED` | `true` | Permit the saved installation setting to enable release checks; false forces checks off |
| `CITADEL_HTTP_OPENAPI_ENABLED` | `false` | Show API reference pages on your installation |
| `CITADEL_STATS_POLL_INTERVAL_SECONDS` | `10` | Collect metrics every 10 seconds |
| `CITADEL_STATS_FLUSH_INTERVAL_SECONDS` | `60` | Save buffered metrics every 60 seconds |
| `CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES` | `15` | Access-token lifetime |
| `CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS` | `30` | Refresh-token lifetime |
| `CITADEL_SCRIPTS_ENABLED` | `true` | Allow script execution |
| `CITADEL_SCRIPTS_MAX_PARALLEL_RUNS` | `4` | Limit script runs executing locally on Core; remote runs have a separate limit |

Leave `CITADEL_JWT_KEY` and `CITADEL_SECRETS_ENCRYPTION_KEY` unset to let Citadel generate
and save them in `citadel_data`. `CITADEL_JWT_ISSUER` and `CITADEL_JWT_AUDIENCE` normally
follow the public URL when not explicitly configured. Back up the data volume
with PostgreSQL so these keys remain available during recovery.

For additional options, use the [environment variable reference](/reference/environment-variables).
Unattended administrator setup is explained in
[first-run setup](/getting-started/first-run-setup).

## Allow update checks

Update checks are **on by default**. Choose **Check for Citadel updates** during
first-run setup or change it in **Administration → Updates → Config** as an
administrator and select **Save**. This choice applies to the whole installation
and takes effect without restarting Core.

To prohibit release checks through deployment configuration, set this in `.env`
and [apply the setting change](#apply-a-setting-change):

```dotenv
CITADEL_UPDATE_CHECKS_ENABLED=false
```

After setup completes, Core checks Citadel's public GitHub release information
when automatic checks are enabled. In **Administration → Updates → Config**, choose
**Every 6 hours** (the default), **Every 12 hours**, or **Daily**, then select **Save**.
The frequency is saved for the installation and takes effect without restarting
Core. The next check is based on the last completed attempt, including manual
checks; restarting Core preserves the schedule. Administrators see a small dot beside the sidebar version and a
notice on Home when a newer release in the installed channel is available.
Stable installations track stable releases; development versions (`-dev.N`)
track development releases. Both notifications open **Administration → Updates**, where
you can read release notes, see the last successful check, and open the
[upgrade guide](/operations/upgrade-and-rollback).
Checks do not install updates. With `CITADEL_UPDATE_CHECKS_ENABLED=false`, the UI
control is disabled and explains that server configuration prevents checks. When
the variable is true or unset, the saved installation choice controls checks.
**Check now** performs a single check even when automatic checks are off, provided
server configuration permits checks. Failed checks retain the last successful
result and show an error. **Activities** records each discovered release once
and changes to the check setting. Agents and Edge Agents do not make these requests.

**No telemetry is collected by Citadel through update checks.** Requests send no
installed version, instance identifiers, configuration, credentials, or usage
data. Version comparison happens inside Core. GitHub receives normal HTTPS
connection information, including your server's outbound IP address. The browser
reads Core's cached result initially, then refreshes it when Core sends a realtime
change notification or the realtime connection reconnects. Opening release notes or the guide visits the linked website.

Checks require outbound HTTPS to `api.github.com`. A failed or rate-limited
request does not interrupt Citadel; the last successful result is kept until the
next successful check or a Core restart.

## Apply a setting change

Save `.env`, then run from your installation directory:

```bash
docker compose up -d
docker compose ps
```

Include your proxy, TLS, and Edge overlays if you use them. Changing an environment variable
requires recreating the container; `docker compose restart` alone does not
load the new value. Keep `.env`, certificates, and bootstrap password files
out of screenshots and shared support files.
