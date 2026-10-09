---
title: "TLS and secure Agent transport"
description: "Secure Citadel API, Agent, and Edge Agent communication."
---

Citadel transports browser sessions, Docker information, logs, configuration,
and Agent commands. Use TLS whenever that traffic leaves one trusted machine or
private network.

For the complete `.env` preparation workflow and application-setting reference,
see [Docker Compose configuration](/docs/getting-started/configuration).

Citadel supports three Core transport modes:

| Mode | Use it for | Public traffic |
| --- | --- | --- |
| `ReverseProxy` | Recommended production setup | TLS ends at your reverse proxy |
| `Direct` | Production without a reverse proxy | TLS ends at Citadel Core |
| `Disabled` | Local development and isolated tests | Plain HTTP |

Request signing and Edge Agent enrollment authenticate Citadel components, but
they do not encrypt traffic. Keep those protections enabled and add TLS.

TLS for regular Platform Agents and inbound Build Pool Agents is optional.
Their default setup authenticates Core requests over HTTP to avoid requiring
certificate management on every Docker host. HTTP does not protect Agent
responses or traffic confidentiality, so use that default only on a trusted
private network or VPN. The Core UI/API and Edge Agent transport still follow
the Core transport mode selected below.

## Before you start

Choose public addresses for:

- the Citadel UI and API;
- the Edge Agent gRPC endpoint.

Citadel Core keeps two internal listeners:

```text
8000  UI, API, WebSocket live updates, OIDC, webhooks, and health
8001  Edge Agent bidirectional gRPC
```

The public addresses may use one hostname when your proxy routes both
protocols, or separate ports/hostnames:

```text
https://citadel.example.com
https://citadel.example.com:8444
```

Certificate DNS names or IP subject alternative names must match the addresses
that clients actually use.

## Recommended: reverse proxy

Use this mode when you already run Caddy, Traefik, nginx, HAProxy, or another
TLS-capable proxy.

For a complete Compose overlay, proxy configuration, and verification steps,
follow [Reverse proxy with Caddy](/docs/operations/reverse-proxy-caddy).

The topology is:

```text
Browser    -> HTTPS proxy -> Core HTTP/1 port 8000
Edge Agent -> HTTPS proxy -> Core HTTP/2 port 8001
```

Set the Core environment:

```dotenv
Transport__Mode=ReverseProxy
Transport__PublicUrl=https://citadel.example.com
Transport__ApiPort=8000
Transport__EdgeGrpcPort=8001

EdgeAgent__PublicGrpcUrl=https://citadel.example.com

Transport__ForwardedHeaders__KnownProxies=172.30.0.2
Transport__ForwardedHeaders__ForwardLimit=1
AllowedHosts=citadel.example.com;localhost

AgentTransport__AllowInsecure=true
```

Set `Transport__ForwardedHeaders__KnownProxies` to the stable address of the
immediate proxy. Multiple values are comma-separated. Use
`Transport__ForwardedHeaders__KnownNetworks` only for a dedicated, isolated
proxy subnet whose containers cannot be joined by untrusted workloads.
Configure at least one known proxy or known network.

The proxy must:

- send UI, API, WebSocket, OIDC, webhook, and health traffic to Core port `8000`;
- send Edge Agent gRPC traffic to Core port `8001` using HTTP/2/h2c;
- support WebSocket upgrades;
- preserve the public `Host`/HTTP2 `:authority`;
- forward `X-Forwarded-For` and `X-Forwarded-Proto`;
- disable buffering for long-lived gRPC and streaming requests;
- use idle timeouts longer than long-lived Citadel connections.

Keep Core ports `8000` and `8001` on the private proxy network. If a proxy
installed directly on the host requires published ports, bind them to
`127.0.0.1` instead of every interface.

The installation's `docker-compose.yml` publishes Core's listeners on host
loopback ports `18000` and `18001` by default; it does not include a reverse
proxy. A proxy running on the host can use those ports. A containerized proxy
needs a shared private Docker network and upstreams `server:8000` and
`server:8001`; its own `localhost` does not reach Core.

Configure the proxy, trusted proxy addresses, and public URLs before starting
Core in `ReverseProxy` mode. Run from your installation directory:

```bash
docker compose -f docker-compose.yml up -d
```

The Edge Agent service name is:

```text
citadel.edge.v1.EdgeAgentService
```

Use it when your proxy supports service-based gRPC routing. Consult your
proxy's documentation for its exact HTTP/2 upstream syntax.

## Direct Core TLS

Use direct mode when Citadel Core should load and serve your certificate.
Direct mode serves HTTPS only; it does not open a separate HTTP redirect
listener.

Place the certificate and key in `./tls`, or set
`CITADEL_TLS_HOST_DIRECTORY` to another directory. The direct-TLS Compose
overlay mounts it read-only at `/etc/citadel/tls`.

Download the overlay into your installation directory:

```bash
curl -fSL https://raw.githubusercontent.com/Citadel-P/Citadel/main/deploy/compose.direct-tls.yml -o compose.direct-tls.yml
```

Set:

```dotenv
CITADEL_BIND_ADDRESS=0.0.0.0
CITADEL_HTTP_PORT=443
CITADEL_EDGE_PORT=8443

Transport__Mode=Direct
Transport__PublicUrl=https://citadel.example.com
Transport__ApiPort=8000
Transport__EdgeGrpcPort=8001

EdgeAgent__PublicGrpcUrl=https://citadel.example.com:8443
AllowedHosts=citadel.example.com;localhost

Transport__Certificate__Path=/etc/citadel/tls/core-fullchain.pem
Transport__Certificate__PrivateKeyPath=/etc/citadel/tls/core-key.pem

AgentTransport__AllowInsecure=true
```

From your installation directory, include the direct-TLS overlay to mount the certificates.
The base Compose file publishes the configured host ports and checks listener health:

```bash
docker compose \
  -f docker-compose.yml \
  -f compose.direct-tls.yml \
  up -d
```

The health check uses HTTPS over loopback and skips certificate verification.
Its only purpose is to verify that the local Core listener is ready; Core
validates the configured certificate during startup.

The PEM certificate file should contain the leaf certificate and required
intermediate certificates. The private key must match it.

The same Core certificate serves the API and Edge gRPC listeners. It must be
valid for the hosts in both `Transport__PublicUrl` and
`EdgeAgent__PublicGrpcUrl`.

## Local development

Use cleartext only when the network is intentionally isolated:

```dotenv
Transport__Mode=Disabled
Transport__PublicUrl=http://localhost:18000
Transport__ApiPort=8000
Transport__EdgeGrpcPort=8001
EdgeAgent__PublicGrpcUrl=http://localhost:18001
AllowedHosts=localhost;host.docker.internal

AgentTransport__AllowInsecure=true
```

The supplied Compose setup maps the container ports to host ports `18000` and
`18001`, bound to loopback. This gRPC URL is local-only; remote Agents need a
reachable, secured address. Citadel logs a warning while this mode is active. Do not expose this setup to
an untrusted LAN or the internet.

## Secure a regular Agent

Regular Platform Agents and inbound self-managed Build Pool Agents are gRPC
servers. They authenticate Core requests over HTTP by default and require no
certificate variables. To require encrypted connections instead, set
`AgentTransport__AllowInsecure=false` on Core and configure direct TLS on the
Agent's existing port `9000`.

Example Agent environment:

```dotenv
CITADEL_AGENT_TLS_MODE=Direct
CITADEL_AGENT_PORT=9000
CITADEL_AGENT_TLS_CERTIFICATE_PATH=/etc/citadel/tls/agent-fullchain.pem
CITADEL_AGENT_TLS_PRIVATE_KEY_PATH=/etc/citadel/tls/agent-key.pem
HUB_PUBLIC_KEY=the-public-key-shown-by-citadel
```

Mount the files and publish the Agent. Replace `AGENT_IMAGE` with the complete
image address from this Platform's generated installation command, and replace
the example paths and public key with your own:

```bash
docker run -d \
  --name citadel-agent \
  --restart=always \
  --label com.citadel.system=true \
  --label com.citadel.system-role=agent \
  -p 9000:9000 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /:/host:ro \
  -v /path/to/agent-tls:/etc/citadel/tls:ro \
  -e CITADEL_AGENT_TLS_MODE=Direct \
  -e CITADEL_AGENT_PORT=9000 \
  -e CITADEL_AGENT_TLS_CERTIFICATE_PATH=/etc/citadel/tls/agent-fullchain.pem \
  -e CITADEL_AGENT_TLS_PRIVATE_KEY_PATH=/etc/citadel/tls/agent-key.pem \
  -e HUB_PUBLIC_KEY="..." \
  AGENT_IMAGE
```

Configure the Platform or inbound Build Pool endpoint with the exact DNS name
or IP covered by the certificate:

```text
https://docker-host.internal.example:9000
```

Core still signs every Agent request. TLS does not replace `HUB_PUBLIC_KEY`.

### Private CA for regular Agents

If Agent certificates use a private CA, mount the CA's public certificate into
Core:

```yaml
services:
  server:
    volumes:
      - ./tls/agent-ca.pem:/etc/citadel/tls/agent-ca.pem:ro
```

Set:

```dotenv
AgentTransport__CaCertificatePath=/etc/citadel/tls/agent-ca.pem
```

The bundle may contain multiple CA certificates. Citadel still validates the
Agent hostname, validity dates, chain, and server-authentication usage.

## Secure an Edge Agent

Edge Agents do not need server certificates because they open the connection
to Core.

For a publicly trusted Core or reverse-proxy certificate:

```dotenv
CITADEL_CORE_URL=https://citadel.example.com
```

Use the URL generated by Citadel enrollment. It points to
`EdgeAgent__PublicGrpcUrl`, which may differ from the browser URL.

### Private CA for Edge Agents

Mount only the CA public certificate:

```bash
-v /path/to/core-ca.pem:/etc/citadel/tls/core-ca.pem:ro
```

Add:

```dotenv
CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH=/etc/citadel/tls/core-ca.pem
```

Do not mount a CA private key into an Agent.

For cleartext communication on a trusted private network, use:

```dotenv
CITADEL_CORE_URL=http://host.docker.internal:8001
```

The URL scheme selects the transport. `http://` enables cleartext HTTP/2;
`https://` enables TLS. Citadel never bypasses certificate validation for an
HTTPS URL.

## Related URL settings

Keep these values consistent with the public deployment:

```dotenv
Jwt__Issuer=https://citadel.example.com
Jwt__Audience=https://citadel.example.com
```

When explicit CORS origins are needed, configure them by index:

```dotenv
Cors__0=https://citadel.example.com
```

Changing JWT issuer or audience invalidates existing access and refresh tokens.
Users must sign in again.

OIDC providers must register Citadel's HTTPS callback:

```text
https://citadel.example.com/api/v1/authentication/oidc/<provider-id>/callback
```

## Certificate renewal

Citadel does not issue or renew certificates.

To renew a direct Core or regular Agent certificate:

1. Replace the mounted certificate and key files.
2. Restart the affected container.
3. Verify its health and connection status.

Citadel warns at startup when a direct certificate expires within 30 days.

## Troubleshooting

### Core rejects proxy configuration

Confirm that `Transport__ForwardedHeaders__KnownProxies` or
`Transport__ForwardedHeaders__KnownNetworks` identifies the immediate proxy-to-
Core connection. Do not enter public client networks.

### OIDC redirects to HTTP

Confirm:

- `Transport__PublicUrl` uses HTTPS;
- Core is in `ReverseProxy` mode;
- the immediate proxy is trusted;
- the proxy sends `X-Forwarded-Proto: https`.

### Edge Agent reports an HTTP/2 error

Verify that `CITADEL_CORE_URL` points to the public Edge gRPC endpoint, not
automatically to the UI endpoint. Confirm that the proxy forwards the Edge
Agent gRPC service to Core port `8001` without HTTP/1.1 downgrade or buffering.

### Certificate is not trusted

For a public certificate, verify that the full intermediate chain is served.
For a private CA, mount the CA public certificate into the correct client:

- Agent CA into Core for regular Agents;
- Core/proxy CA into Edge Agents.

### Certificate name mismatch

The hostname or IP in the configured URL must appear in the certificate subject
alternative names. Changing only the URL scheme does not fix a name mismatch.

### Agent endpoint is rejected as insecure

Use an `https://` Agent endpoint, or set
`AgentTransport__AllowInsecure=true` when the Agent is reachable only through
a trusted private network or VPN.

Citadel does not provide a certificate-validation bypass.
