---
title: "Reverse proxy with Caddy"
description: "Serve Citadel and Edge Agent connections over HTTPS using Docker Compose and Caddy."
---

This example adds Caddy to the [standard Compose installation](/docs/getting-started/install).
One hostname serves the browser, API, and Edge Agent gRPC endpoint. Caddy handles
certificates; Citadel's internal listeners remain on a private Docker network.

## Before you start

- Prepare the installation's `docker-compose.yml` and `.env`, including its
  database credentials, secret keys, and published Citadel image.
- Replace `citadel.example.com` below with a hostname you control. Its DNS records
  must point to this Docker host, including any IPv6 record you publish.
- Make TCP ports `80` and `443` available to Caddy and reachable for certificate
  validation. This example assumes Caddy is the public-facing proxy.
- Choose an unused Docker subnet. The example uses `172.30.250.0/29`; change its
  addresses together if they overlap with your existing networks.

For private certificates or another proxy, use the
[transport guide](/docs/operations/tls-and-secure-agent-transport).

## 1. Configure Citadel

Update these entries in the installation's `.env`, keeping its other settings:

```dotenv
CITADEL_BIND_ADDRESS=127.0.0.1
CITADEL_HTTP_PORT=18000
CITADEL_EDGE_PORT=18001

Transport__Mode=ReverseProxy
Transport__PublicUrl=https://citadel.example.com
Transport__ApiPort=8000
Transport__EdgeGrpcPort=8001
Transport__ForwardedHeaders__KnownProxies=172.30.250.2
Transport__ForwardedHeaders__ForwardLimit=1
AllowedHosts=citadel.example.com;localhost
EdgeAgent__PublicGrpcUrl=https://citadel.example.com

Jwt__Issuer=https://citadel.example.com
Jwt__Audience=https://citadel.example.com
```

Clear any existing `Transport__Certificate__Path`,
`Transport__Certificate__PrivateKeyPath`, and
`Transport__ForwardedHeaders__KnownNetworks` values. This example trusts only
Caddy's fixed address. Changing the JWT issuer or audience signs existing users
out; they can sign in again at the HTTPS address.

If you configured explicit `Cors__*` origins, update them for the HTTPS address.
For OIDC sign-in, also update the provider's registered callback URL; see
[related URL settings](/docs/operations/tls-and-secure-agent-transport#related-url-settings).

## 2. Add Caddy

Create `compose.caddy.yml` beside `docker-compose.yml`:

```yaml
services:
  server:
    networks:
      default: {}
      citadel_proxy:
        ipv4_address: 172.30.250.3

  caddy:
    image: caddy:2
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    networks:
      citadel_proxy:
        ipv4_address: 172.30.250.2

networks:
  citadel_proxy:
    ipam:
      config:
        - subnet: 172.30.250.0/29

volumes:
  caddy_data:
  caddy_config:
```

The `default` network keeps Core connected to PostgreSQL. Core's published ports
remain bound to host loopback; Caddy reaches its container ports directly.
Keep the proxy network dedicated to these services and retain `caddy_data`
across upgrades so certificate state persists.

Create `Caddyfile` in the same directory:

```text
citadel.example.com {
    @edge path /citadel.edge.v1.EdgeAgentService/*
    handle @edge {
        reverse_proxy h2c://server:8001
    }

    handle {
        reverse_proxy server:8000
    }
}
```

The Edge route preserves the gRPC service path and uses cleartext HTTP/2 only
inside the proxy network. Caddy's [reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
handles WebSocket upgrades and forwarding headers for browser traffic.
[Automatic HTTPS](https://caddyserver.com/docs/automatic-https) obtains and renews
the public certificate.

## 3. Validate and start

Run from the installation directory:

```bash
docker compose -f docker-compose.yml -f compose.caddy.yml config --quiet
docker compose -f docker-compose.yml -f compose.caddy.yml pull
docker compose -f docker-compose.yml -f compose.caddy.yml run --rm --no-deps caddy \
  caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
docker compose -f docker-compose.yml -f compose.caddy.yml up -d
docker compose -f docker-compose.yml -f compose.caddy.yml ps
```

Always include both Compose files in later update commands. Follow
[upgrade and rollback](/docs/operations/upgrade-and-rollback) before changing an
existing installation's images.

## 4. Check the connection

1. Open `https://citadel.example.com` and confirm the browser accepts its certificate.
2. Sign in and open a container's **Logs** or **Stats** to check live updates.
3. If using Edge Agents, create or open an Edge Platform and use its generated
   command with `CITADEL_CORE_URL=https://citadel.example.com`. Confirm the
   Platform comes online; an existing Agent using the old URL needs its
   configuration updated and container recreated.

If the browser works but Edge enrollment fails, check the gRPC route and ensure
the Agent uses the HTTPS endpoint. If Core rejects forwarded headers, verify
that Caddy's address matches `KnownProxies`.

For certificate issuance or upstream errors, inspect Caddy's logs:

```bash
docker compose -f docker-compose.yml -f compose.caddy.yml logs --tail=100 caddy
```

Check DNS, inbound ports, and Core health before retrying. See
[transport troubleshooting](/docs/operations/tls-and-secure-agent-transport#troubleshooting)
for certificate trust and hostname errors.
