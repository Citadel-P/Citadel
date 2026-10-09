---
title: "Nginx in Docker"
description: "Serve Citadel and Edge Agent connections over HTTPS using Docker Compose and Nginx."
---

This example adds Nginx to the [standard Compose installation](/docs/getting-started/install).
One hostname serves the browser, API, and Edge Agent gRPC endpoint. TLS ends at
Nginx; traffic to Core stays on a dedicated Docker network.

If Nginx already runs as a system service on the Docker host, use
[Nginx on the host](/docs/operations/reverse-proxy-nginx-host) instead.

## Before you start

- Prepare the installation's `docker-compose.yml` and `.env`.
- Point `citadel.example.com` to your Docker host and make ports `80` and `443`
  available to Nginx. Replace this hostname throughout the example.
- Place a valid PEM certificate chain and matching private key in
  `./tls/fullchain.pem` and `./tls/privkey.pem`. Nginx does not obtain or renew
  certificates automatically; use your certificate provider's renewal process.
- Choose an unused subnet. If `172.30.249.0/29` overlaps your networks, change
  the subnet, container addresses, and `KnownProxies` together.

For automatic certificate management, see
[Reverse proxy with Caddy](/docs/operations/reverse-proxy-caddy).

## 1. Configure Citadel

Update these entries in `.env`, keeping its other settings:

```dotenv
CITADEL_BIND_ADDRESS=127.0.0.1
CITADEL_HTTP_PORT=18000
CITADEL_EDGE_PORT=18001

Transport__Mode=ReverseProxy
Transport__PublicUrl=https://citadel.example.com
Transport__ApiPort=8000
Transport__EdgeGrpcPort=8001
Transport__ForwardedHeaders__KnownProxies=172.30.249.2
Transport__ForwardedHeaders__ForwardLimit=1
AllowedHosts=citadel.example.com;localhost
EdgeAgent__PublicGrpcUrl=https://citadel.example.com

Jwt__Issuer=https://citadel.example.com
Jwt__Audience=https://citadel.example.com
```

Clear any existing `Transport__Certificate__Path`,
`Transport__Certificate__PrivateKeyPath`, and
`Transport__ForwardedHeaders__KnownNetworks` values. Core trusts Nginx's fixed
address. Changing the JWT issuer or audience signs existing users out.

Update explicit CORS origins and OIDC callback URLs if configured; see
[related URL settings](/docs/operations/tls-and-secure-agent-transport#related-url-settings).

## 2. Add Nginx

Create `compose.nginx.yml` beside `docker-compose.yml`:

```yaml
services:
  server:
    networks:
      default: {}
      citadel_proxy:
        ipv4_address: 172.30.249.3

  nginx:
    image: nginx:stable-alpine
    restart: unless-stopped
    depends_on:
      server:
        condition: service_healthy
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/conf.d/default.conf:ro
      - ./tls:/etc/nginx/tls:ro
    networks:
      citadel_proxy:
        ipv4_address: 172.30.249.2

networks:
  citadel_proxy:
    ipam:
      config:
        - subnet: 172.30.249.0/29
```

Core retains its default network for PostgreSQL. Keep the proxy network
dedicated to Core and Nginx; Core's published ports remain bound to loopback.

Create `nginx.conf` in the same directory. The image loads this file inside its
`http` configuration, so it contains no outer `http` or `events` block:

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

server {
    listen 80;
    server_name citadel.example.com;
    return 301 https://citadel.example.com$request_uri;
}

server {
    listen 443 ssl;
    http2 on;
    server_name citadel.example.com;

    ssl_certificate /etc/nginx/tls/fullchain.pem;
    ssl_certificate_key /etc/nginx/tls/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    client_max_body_size 0;

    location ^~ /citadel.edge.v1.EdgeAgentService/ {
        grpc_pass grpc://server:8001;
        grpc_set_header Host $http_host;
        grpc_set_header X-Forwarded-For $remote_addr;
        grpc_set_header X-Forwarded-Proto $scheme;
        grpc_read_timeout 3600s;
        grpc_send_timeout 3600s;
        grpc_intercept_errors off;
    }

    location / {
        proxy_pass http://server:8000;
        proxy_http_version 1.1;
        proxy_set_header Host $http_host;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_buffering off;
        proxy_request_buffering off;
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
        proxy_intercept_errors off;
    }
}
```

The [gRPC route](https://nginx.org/en/docs/http/ngx_http_grpc_module.html#grpc_pass)
preserves the Edge service path and uses HTTP/2 to Core port `8001`.
Browser traffic uses port `8000`, with WebSocket upgrades and response buffering
disabled. Core enforces its own request size limits; Nginx adds no smaller limit.

`X-Forwarded-For` contains the immediate client's address, matching Core's
one-hop trust configuration. This example assumes Nginx is the public-facing
proxy; review forwarding trust before placing another proxy in front of it.

[Error interception](https://nginx.org/en/docs/http/ngx_http_proxy_module.html#proxy_intercept_errors)
is disabled so API failures keep their status code and JSON details. Do not add
an `error_page` maintenance fallback to the Citadel locations.

## 3. Validate and start

Run from the installation directory:

```bash
docker compose -f docker-compose.yml -f compose.nginx.yml config --quiet
docker compose -f docker-compose.yml -f compose.nginx.yml pull
docker compose -f docker-compose.yml -f compose.nginx.yml up -d server
docker compose -f docker-compose.yml -f compose.nginx.yml run --rm --no-deps nginx nginx -t
docker compose -f docker-compose.yml -f compose.nginx.yml up -d
```

Core must be running for Nginx to resolve `server` during validation. Always
include both Compose files in later update commands.

After configuration changes or certificate renewal, validate and reload Nginx:

```bash
docker compose -f docker-compose.yml -f compose.nginx.yml exec nginx nginx -t
docker compose -f docker-compose.yml -f compose.nginx.yml exec nginx nginx -s reload
```

## 4. Check the connection

1. Open `https://citadel.example.com` and confirm its certificate is accepted.
2. Sign in and open a container's **Logs** or **Stats** to check live updates.
3. For Edge Agents, use the Platform's generated command with
   `CITADEL_CORE_URL=https://citadel.example.com` and confirm it comes online.

If Core rejects forwarded headers, check that `KnownProxies` matches Nginx's
address. If browser access works but Edge enrollment fails, check the service
route and HTTP/2 listener. For TLS or upstream errors, inspect Nginx's logs:

```bash
docker compose -f docker-compose.yml -f compose.nginx.yml logs --tail=100 nginx
```

See [transport troubleshooting](/docs/operations/tls-and-secure-agent-transport#troubleshooting)
for certificate trust and hostname errors.
