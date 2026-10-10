---
title: "Docker Compose configuration"
description: "Change Citadel settings without losing your installation data."
---

Citadel's installation settings live in `.env` beside `docker-compose.yml`
in your installation directory. Application settings,
such as Platforms and Deployments, are managed in the browser.

## Prepare the environment

The [installation guide](/docs/getting-started/install) downloads the environment
template as `.env`. Open that file in a text editor to change settings.

Enter the database password without surrounding quotes. A long, randomly generated
password containing letters and numbers avoids environment-file parsing problems.

Do this only for a new installation. Keep your existing `.env` when upgrading.
The example is a **local HTTP setup**, not a ready-made public installation.

## Settings most installations need

| Setting | Example default | When to change it |
| --- | --- | --- |
| `PG_PASSWORD` | Empty | Always set a long, unique database password before starting |
| `CITADEL_IMAGE` | Empty → `ghcr.io/citadel-p/citadel:latest` | Optional: pin a specific version or use another registry |
| `CITADEL_BIND_ADDRESS` | `127.0.0.1` | Change only after configuring secure network access |
| `CITADEL_HTTP_PORT` | `18000` | Change if this host port is already in use |
| `CITADEL_EDGE_PORT` | `18001` | Host port for the separate Edge Agent connection |
| `Transport__Mode` | `Disabled` | Choose `ReverseProxy` or `Direct` for HTTPS network access |
| `Transport__PublicUrl` | `http://localhost:18000` | Set to the browser address users will open |
| `EdgeAgent__PublicGrpcUrl` | `http://localhost:18001` | Set to the reachable gRPC address for Edge Agents |
| `AllowedHosts` | Local hostnames | Include the hostname of your shared installation |

The supplied PostgreSQL values are `PG_HOST=pg_db`, `PG_PORT=5432`,
`PG_USER=citadel`, and `PG_DATABASE=citadel`. These match the Compose database
service. Do not change them independently on an existing installation.

## Transport mode

- **Disabled**: HTTP for local evaluation on loopback or an isolated test network.
- **ReverseProxy**: an HTTPS proxy sits in front of Citadel. Configure the immediate trusted proxy addresses and public URLs.
- **Direct**: Citadel serves TLS. Configure certificate and key paths, then include `compose.direct-tls.yml`.

The complete instructions are in
[TLS and secure Agent transport](/docs/operations/tls-and-secure-agent-transport).
Browser traffic and Edge Agent traffic use separate listeners. Their container
ports (`8000` and `8001`) differ from the default host ports (`18000` and `18001`).

## Optional settings

| Setting | Default | Purpose |
| --- | --- | --- |
| `Passwords__MinimumLength` | `15` | Minimum length for new or changed passwords; accepts 8–128 characters, with a fixed maximum password length of 128 |
| `Mfa__Policy` | `Optional` | Require two-factor authentication for administrators or all users |
| `Updates__Enabled` | `true` | Allow Core to check for new Citadel releases in its installed channel |
| `EnableSwagger` | `false` | Show API reference pages on your installation |
| `JobConfiguration__MonitoringInterval` | `10` | Collect metrics every 10 seconds |
| `JobConfiguration__FlashInterval` | `60` | Save buffered metrics every 60 seconds |
| `Jwt__AccessToken__ValidForMinutes` | `15` | Access-token lifetime |
| `Jwt__RefreshToken__ValidForDays` | `30` | Refresh-token lifetime |
| `Automations__Enabled` | `true` | Allow automation execution |
| `Automations__MaxParallelRuns` | `4` | Limit concurrent automation runs |

Leave `Jwt__Key` and `Secrets__EncryptionKey` unset to let Citadel generate
and save them in `citadel_data`. `Jwt__Issuer` and `Jwt__Audience` normally
follow the public URL when not explicitly configured. Back up the data volume
with PostgreSQL so these keys remain available during recovery.

For additional options, use the [environment variable reference](/docs/reference/environment-variables).
Unattended administrator setup is explained in
[first-run setup](/docs/getting-started/first-run-setup).

## Allow update checks

Update checks are **on by default**. To disable them, set this in `.env` and
[apply the setting change](#apply-a-setting-change):

```dotenv
Updates__Enabled=false
```

Core checks Citadel's public GitHub release information at startup and every
six hours. Administrators see a small dot beside the sidebar version and a
notice on Home when a newer release in the installed channel is available.
Stable installations track stable releases; development versions such as
`0.1.4-dev.1` track development releases. Open either notification for
release notes and the [upgrade guide](/docs/operations/upgrade-and-rollback).
Checks do not install updates. Set `Updates__Enabled=true` to re-enable them
after opting out. Agents and Edge Agents do not make these requests.

**No telemetry is collected by Citadel through update checks.** Requests send no
installed version, instance identifiers, configuration, credentials, or usage
data. Version comparison happens inside Core. GitHub receives normal HTTPS
connection information, including your server's outbound IP address. The browser
only reads Core's cached result; opening release notes or the guide visits the
linked website.

Checks require outbound HTTPS to `api.github.com`. A failed or rate-limited
request does not interrupt Citadel; the last successful result is kept until the
next successful check or a Core restart.

## Apply a setting change

Save `.env`, then run from your installation directory:

```bash
docker compose up -d
docker compose ps
```

Include your TLS overlay if you use one. Changing an environment variable
requires recreating the container; `docker compose restart` alone does not
load the new value. Keep `.env`, certificates, and bootstrap password files
out of screenshots and shared support files.
