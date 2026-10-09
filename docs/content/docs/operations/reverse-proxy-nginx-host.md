---
title: "Nginx on the host"
description: "Use a host-installed Nginx server for Citadel HTTPS, WebSockets, and Edge Agent gRPC."
---

Use this setup when Nginx already runs as a system service on the same Linux
host as Citadel. Core stays in Docker; Nginx connects to its published loopback
ports. For a containerized proxy, use [Nginx in Docker](/docs/operations/reverse-proxy-nginx).

## Before you start

- Prepare the [standard Compose installation](/docs/getting-started/install).
- Install Nginx with HTTP/2 and gRPC support. The configuration below uses
  Nginx 1.25.1 or later.
- Point `citadel.example.com` to this host and allow public traffic on ports
  `80` and `443`. Replace the example hostname throughout.
- Obtain a valid certificate chain and matching private key using your
  certificate provider. The example uses existing files under
  `/etc/letsencrypt/live/citadel.example.com/`; change these paths if needed.

## 1. Configure Citadel

Core must trust the proxy's address **as seen inside its container**. With a
Linux Docker bridge, host connections commonly arrive from the bridge gateway.
From the installation directory, inspect the running Core container:

```bash
docker inspect "$(docker compose ps -q server)" \
  --format '{{range $name, $network := .NetworkSettings.Networks}}{{$name}} gateway={{$network.Gateway}}{{println}}{{end}}'
```

Use the installation's bridge gateway for `KnownProxies` below; `172.19.0.1`
is an example, not a fixed installation default. With
[rootless Docker](https://docs.docker.com/engine/security/rootless/troubleshoot/#networking-errors),
the source address also depends on the port-forwarding driver. If Core rejects
forwarded headers, confirm the actual peer using the diagnostic below. Do not
assume it is `127.0.0.1` just because Nginx's upstream uses loopback.

Update `.env`, keeping its other settings:

```dotenv
CITADEL_BIND_ADDRESS=127.0.0.1
CITADEL_HTTP_PORT=18000
CITADEL_EDGE_PORT=18001

Transport__Mode=ReverseProxy
Transport__PublicUrl=https://citadel.example.com
Transport__ApiPort=8000
Transport__EdgeGrpcPort=8001
Transport__ForwardedHeaders__KnownProxies=172.19.0.1
Transport__ForwardedHeaders__ForwardLimit=1
AllowedHosts=citadel.example.com;localhost
EdgeAgent__PublicGrpcUrl=https://citadel.example.com

Jwt__Issuer=https://citadel.example.com
Jwt__Audience=https://citadel.example.com
```

Clear existing `Transport__Certificate__Path`,
`Transport__Certificate__PrivateKeyPath`, and
`Transport__ForwardedHeaders__KnownNetworks` values. Update explicit CORS origins
and OIDC callback URLs if configured; see
[related URL settings](/docs/operations/tls-and-secure-agent-transport#related-url-settings).
Changing the JWT issuer or audience signs existing users out.

Apply the settings and confirm the private listener works:

```bash
docker compose up -d --wait
curl --fail --header 'Host: citadel.example.com' http://127.0.0.1:18000/health
```

If your installation uses different host ports, keep the `.env` values and
Nginx upstreams below consistent. Run Docker commands as the account that owns
the installation's Docker engine, including for rootless deployments.

## 2. Configure Nginx

Create `/etc/nginx/conf.d/citadel.conf`, or update your existing Citadel virtual
host. This file belongs inside Nginx's `http` block; the usual Linux package
configuration already includes `conf.d/*.conf` there.

```nginx
map $http_upgrade $citadel_connection_upgrade {
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

    ssl_certificate /etc/letsencrypt/live/citadel.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/citadel.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    client_max_body_size 0;

    location ^~ /citadel.edge.v1.EdgeAgentService/ {
        grpc_pass grpc://127.0.0.1:18001;
        grpc_set_header Host $http_host;
        grpc_set_header X-Forwarded-For $remote_addr;
        grpc_set_header X-Forwarded-Proto $scheme;
        grpc_read_timeout 3600s;
        grpc_send_timeout 3600s;
        grpc_intercept_errors off;
    }

    location / {
        proxy_pass http://127.0.0.1:18000;
        proxy_http_version 1.1;
        proxy_set_header Host $http_host;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $citadel_connection_upgrade;
        proxy_buffering off;
        proxy_request_buffering off;
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
        proxy_intercept_errors off;
    }
}
```

The [HTTP/2 directive](https://nginx.org/en/docs/http/ngx_http_v2_module.html#http2)
was introduced in Nginx 1.25.1. For an older distribution package, replace
`listen 443 ssl;` and `http2 on;` with `listen 443 ssl http2;`.

Browser traffic uses host port `18000`; Edge gRPC uses `18001`. Keep both bound
to loopback. The proxy preserves API error responses, supports WebSockets, and
uses long idle timeouts. Avoid an `error_page` maintenance fallback for these
locations. This example assumes Nginx is the public-facing proxy; review
forwarding trust if another proxy sits in front of it.

## 3. Validate and reload

```bash
sudo nginx -t
sudo systemctl reload nginx
```

Repeat these commands after certificate renewal or configuration changes. Keep
certificate issuance and renewal in your existing certificate-management process.

Open `https://citadel.example.com`, sign in, and check a container's **Logs** or
**Stats**. For an Edge Platform, use its generated Agent command with
`CITADEL_CORE_URL=https://citadel.example.com` and confirm it comes online.

For upstream errors, check Core health and `/var/log/nginx/error.log`. If only
Edge enrollment fails, check the service route and HTTP/2 support.

## Diagnose a rejected proxy address

If Core reports **Invalid forwarded headers**, verify the peer instead of
trusting an entire Docker subnet. A temporary listener on the same bridge can
show the address used by host port forwarding. Replace `citadel_default` with
the network name printed in step 1, and use an unused loopback port:

```bash
docker run --rm --network citadel_default -p 127.0.0.1:18002:8000 \
  python:3.13-alpine python -u -c '
import socket
with socket.socket() as listener:
    listener.bind(("0.0.0.0", 8000))
    listener.listen(1)
    connection, peer = listener.accept()
    with connection:
        connection.recv(4096)
        print("Proxy peer:", peer[0])
        connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
'
```

In another terminal on the same host, run `curl http://127.0.0.1:18002/`. The
listener prints the peer and exits; Docker removes the temporary container.
Set `KnownProxies` to that address and recreate Core with `docker compose up -d`.
Verify it again if you recreate the Docker network or change rootless networking.
