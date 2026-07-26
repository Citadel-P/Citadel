# Edge Agent

Edge Agent connects a Docker host to Citadel without exposing an inbound agent port.

Use Edge Agent when Citadel Core cannot directly reach the Docker host, such as:

- a remote site behind NAT
- a home lab or branch network without inbound firewall rules
- a host that can make outbound HTTPS requests but should not accept management traffic

In Edge Agent mode, the Citadel Agent starts the connection to Citadel Core and keeps an outbound gRPC stream open. Citadel sends Docker operations over that active connection.

## When To Use Edge Agent

Use Edge Agent when opening an inbound port to the agent is inconvenient or unsafe.

Use the regular inbound agent when Citadel Core can reliably reach the agent address and you prefer a direct Core-to-agent connection.

Edge Agent still requires:

- the agent host can reach Citadel Core
- the agent container can access the Docker socket
- the agent data directory is persisted
- Citadel Core is reachable at the URL used during enrollment

For production, run Citadel Core over HTTPS.

## Builds

Edge-agent platforms can run Citadel build projects over the outbound edge-agent transport.

Citadel Core resolves the Git repository, packages the selected build context, and sends it through the edge-agent stream. The edge agent then streams the context to Docker and pushes the configured image tags.

The packaged context must stay under the current `16 MB` envelope limit. Add a `.dockerignore` file to the build context and prefer narrow monorepo service directories over repository-root contexts.

Because edge agents use an outbound command channel, cancelling a build closes the active build stream and the Docker Engine API build is cancelled when Docker observes the dropped request.

Build Pools can also use Edge Agent mode, but that flow is pool-scoped. Create a self-managed Build Pool, select `Connection mode: Edge Agent`, save the pool, then generate the enrollment command from the pool configuration. A build-pool Edge Agent does not require an Edge Agent platform resource.

## Production gRPC Endpoint

Edge Agent uses bidirectional gRPC over HTTP/2. The URL in `CITADEL_CORE_URL` must point to the Citadel Edge Agent gRPC endpoint, not the normal HTTP API endpoint.

In production:

- use HTTPS for the public Edge Agent gRPC URL
- make sure the reverse proxy supports HTTP/2 gRPC upstreams
- do not downgrade the connection to HTTP/1.1
- allow long-lived streaming requests
- keep request and response buffering disabled for the gRPC route
- use idle timeouts longer than the agent keepalive interval

If the proxy does not support gRPC over HTTP/2, the agent may connect and then fail with HTTP/2 protocol errors.

## Create An Edge Platform

Open:

```text
Platforms -> Add Platform
```

Choose:

- `Connection`: `Edge Agent`
- `Name`: a clear host or site name
- `Tags`: optional tags for grouping and filtering

Select:

```text
Create and Enroll
```

Citadel creates the platform and generates an enrollment token.

The token is shown once and stored only as a hash. Copy the generated Docker command or the environment values before leaving the page.

## Run The Agent

Run the command shown by Citadel on the Docker host you want to manage.

The command includes environment variables similar to:

```env
CITADEL_AGENT_MODE=edge
CITADEL_EDGE_AGENT_PROFILE=edge-agent
CITADEL_CORE_URL=https://citadel.example.com
CITADEL_EDGE_ENROLLMENT_TOKEN=...
CITADEL_EDGE_AGENT_KEY_PATH=/app/data/edge-agent.key
CITADEL_EDGE_IDENTITY_PATH=/app/data/edge-agent.identity.json
```

The container should mount:

```text
/var/run/docker.sock:/var/run/docker.sock
/:/host:ro
citadel_edge_agent_data:/app/data
```

The Docker socket lets the agent inspect and manage local Docker resources. The
read-only `/host` bind lets a Platform Edge Agent report the capacity of the
filesystem containing Docker's configured data root. It exposes host paths to
the trusted Agent process but does not allow writes through the mount. Removing
it disables only disk metrics. Upgrade and recreate older Platform Edge Agent
containers to add the mount. For ZFS, the metric represents the mounted dataset
or filesystem, not total pool allocation. Build-pool-only Edge Agents do not
receive this mount.

The data volume stores the agent key and enrolled identity.

Do not remove the persistent data volume unless you intend to re-enroll the agent.

For local Docker testing, Citadel Core exposes the Edge Agent gRPC endpoint on port `8001`. In that setup, use:

```env
CITADEL_CORE_URL=http://host.docker.internal:8001
```

Port `8000` is the normal Citadel HTTP API and UI endpoint. Edge Agent gRPC uses HTTP/2 and should connect to the configured gRPC endpoint.

## Enrollment

On first start, the agent:

1. Generates an agent key pair.
2. Connects to Citadel Core.
3. Sends the enrollment token and public key.
4. Receives an agent identity from Citadel.
5. Stores the identity and private key in the configured data paths.

After enrollment, the enrollment token is no longer needed.

If the token expires before the agent connects, generate a new token from the platform page and restart the agent with the new token.

## Reconnects

After enrollment, the agent reconnects with its saved identity.

Citadel verifies reconnects with a cryptographic challenge. The agent signs the challenge with its private key, and Citadel verifies it with the public key saved during enrollment.

Because of this, the data volume is important. If the agent loses its key or identity file, Citadel will not recognize it as the same enrolled agent.

Reconnects use exponential backoff with jitter. Short network interruptions should recover automatically without re-enrollment.

## Platform Status

An Edge Agent platform is online when the agent has an active connection to Citadel Core.

The agent sends regular heartbeats with:

- Docker reachability
- Docker version
- hostname
- agent version
- supported capabilities

If the agent stops, loses network access, or cannot reach Citadel Core, the platform becomes unavailable for operations that need the agent.

Citadel also validates the agent protocol version and advertised capabilities during enrollment and reconnect. If the agent image is too old for the Core version, reconnect is rejected with a clear session rejection message.

## Docker Operations

When you use an Edge Agent platform, Citadel routes supported Docker operations over the agent's active connection.

For example:

1. You open containers for the platform.
2. Citadel sends a containers command over the Edge Agent stream.
3. The agent runs the Docker operation locally.
4. The agent streams the result back to Citadel.

The Edge Agent does not give Citadel shell access to the host. It only handles the supported agent commands.

Supported operations include platform status, container list/logs/stats/inspect/create/start/stop/restart/delete/exec, image list/inspect/pull/build/push/delete, build-host checks, volume list/inspect/create/delete, network list/inspect/create/delete, stack apply, and deployment apply.

## Rotate Enrollment

Generate a new enrollment token when:

- the first token expired before use
- the token was copied to the wrong place
- the platform was created but the agent was never enrolled

Creating a new token does not rotate an already enrolled agent identity. For an enrolled agent, keep the persistent data volume.

## Revoke Access

Use revoke when the agent identity should no longer connect.

Revoking:

- marks the Edge Agent binding as revoked
- disconnects the active session
- prevents reconnect with the stored identity

After revocation, start over with a new enrollment if the host should be connected again.

## Troubleshooting

`Platform stays offline`

Check that the agent container is running and can reach `CITADEL_CORE_URL`.

`Enrollment token is rejected`

The token may be expired, already used, copied incorrectly, or revoked. Generate a new token and restart the agent with the new value.

`Agent reconnects as a new install`

The data volume may not be persisted. Keep the `/app/data` mount so the key and identity survive container restarts.

`Docker operations fail`

Check that the container has access to `/var/run/docker.sock` and that Docker is running on the host.

`Core URL works in a browser but the agent cannot connect`

The agent container must be able to reach the URL from inside Docker. Use the externally reachable Citadel URL, not a browser-only or machine-local address.

`HTTP/2 server closed the connection`

The Edge Agent gRPC endpoint is probably being served through an HTTP/1.1-only route or proxy. Point `CITADEL_CORE_URL` at the HTTP/2 gRPC endpoint and configure the proxy for gRPC pass-through or gRPC upstream forwarding.

## Security Notes

- Use HTTPS for Citadel Core in production.
- Treat enrollment tokens as short-lived secrets.
- Keep the agent data volume private.
- Revoke an agent if its host or data volume is compromised.
- Do not expose the Docker socket to containers you do not trust.
