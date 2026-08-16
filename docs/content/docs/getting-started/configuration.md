---
title: "Docker Compose configuration"
description: "Configure a production Citadel installation deployed with Docker Compose."
---

Citadel ships `.env.example` as a production-oriented configuration template.
Do not run a production installation directly from that file and do not store
deployment secrets in the repository.

## Prepare the environment

Create the local configuration file:

```bash
cp .env.example .env
chmod 600 .env
```

On PowerShell:

```powershell
Copy-Item .env.example .env
```

Replace the active hostname and secret placeholders before starting Citadel.
Optional settings are commented out so the initial configuration stays small.
Uncomment one only when you need to override its documented default. The `.env`
file is ignored by Git and is read by `docker-compose.yml`.

## Required production values

The active settings in `.env.example` are the values required by the selected
reverse-proxy setup:

| Setting | Purpose |
| --- | --- |
| `PG_USER` | PostgreSQL user |
| `PG_PASSWORD` | PostgreSQL password |
| `PG_DATABASE` | PostgreSQL database |
| `Transport__Mode` | Production transport mode |
| `Transport__PublicUrl` | Browser and API origin |
| `EdgeAgent__PublicGrpcUrl` | Public Edge Agent gRPC origin |
| `AllowedHosts` | Hosts accepted by ASP.NET Core; keep `localhost` for health checks and internal automation calls |
| `Transport__ForwardedHeaders__KnownProxies` | Immediate reverse-proxy address |
| `Jwt__Issuer` | JWT issuer, normally the public Citadel origin |
| `Jwt__Audience` | JWT audience, normally the public Citadel origin |

Generate a random database password. Do not reuse it for the initial
administrator password.

## Transport mode

The example selects `ReverseProxy`, the recommended production mode. Configure
the immediate proxy address or network and keep Core's cleartext listeners on
the private proxy network.

For direct TLS, change `Transport__Mode` to `Direct`, remove the
`Transport__ForwardedHeaders__*` values, configure the PEM certificate and key
paths, and start the direct-TLS overlay:

```bash
docker compose \
  -f docker-compose.yml \
  -f docker-compose.direct-tls.yml \
  up -d
```

Use `Disabled` only for local development or an intentionally isolated test
network. See [TLS and secure Agent transport](/docs/operations/tls-and-secure-agent-transport)
for complete proxy, direct TLS, and Agent TLS instructions.

## Application settings

The template also documents:

| Setting | Default | Purpose |
| --- | ---: | --- |
| `CITADEL_IMAGE_TAG` | Required for release installs | Exact Core image version or digest; Citadel does not publish `latest` |
| `PG_HOST` | `pg_db` | PostgreSQL host |
| `JobConfiguration__MonitoringInterval` | `10` | Metric collection interval in seconds |
| `JobConfiguration__FlashInterval` | `60` | Buffered metric persistence interval in seconds |
| `Jwt__AccessToken__ValidForMinutes` | `15` | Access-token lifetime |
| `Jwt__RefreshToken__ValidForDays` | `30` | Refresh-token lifetime |
| `Jwt__Key` | Generated | JWT signing key |
| `Secrets__EncryptionKey` | Generated | Encryption key for stored secrets and provider tokens |
| `Mfa__Policy` | `Optional` | MFA enforcement policy |
| `Cors__0` | Same-origin only | Additional browser origin allowed by CORS |
| `EnableSwagger` | `false` | Swagger/OpenAPI exposure |
| `EnableLogColor` | `false` | ANSI coloring in container logs |
| `Automations__Enabled` | `true` | Automation execution |
| `Automations__MaxParallelRuns` | `4` | Maximum concurrent automation runs |
| `Automations__InternalBaseUrl` | `http://localhost:8000` | Loopback API URL used by automation actions |

Leave `Jwt__Key` and `Secrets__EncryptionKey` commented to let Citadel generate
and persist them in the Core data volume. Back up that volume with the database
because sessions and encrypted stored values cannot be recovered without those
keys.

Unattended first-run setup is optional. If enabled, mount the administrator
password file read-only rather than putting the password in `.env`. See
[First-Run Setup](/docs/getting-started/first-run-setup).

## Start and update

Start the production services without the development override:

```bash
docker compose -f docker-compose.yml up -d
```

After changing `.env`, apply the configuration again:

```bash
docker compose -f docker-compose.yml up -d
```

Keep `.env` out of source control, support bundles, screenshots, and shared
archives. TLS private keys and bootstrap password files should also remain
outside the repository and be mounted read-only.


