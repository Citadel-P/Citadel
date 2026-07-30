# Connect a Docker host with the regular Agent

The regular Citadel Agent connects a Docker host to Citadel Core through a direct, inbound connection.

Use the regular Agent when:

* Citadel Core can reach the Docker host over the network.
* Opening an inbound port on the Docker host is acceptable.
* You want Citadel Core to initiate requests directly to the Agent.

Use an **Edge Agent** instead when the Docker host is behind NAT, protected by a restrictive firewall, or located on a network where opening an inbound port is not practical.

## How it works

The regular Agent runs on the Docker host and listens for requests from Citadel Core.

Citadel Core signs every Agent request using its private signing key. The Agent validates the signature using the public key provided through the `HUB_PUBLIC_KEY` environment variable.

The private key remains on Citadel Core and must never be copied to an Agent. Agents receive only the corresponding public key.

> Request signing authenticates Citadel Core and protects requests from
> tampering. It does not encrypt network traffic. Certificates are optional.
> Use a private network for the default HTTP setup, or enable TLS when the
> connection crosses an untrusted network.

For production Core and Agent TLS configuration, including private certificate
authorities, see `docs/user/tls-and-secure-agent-transport.md`.

Regular Agents use HTTP by default to keep installation simple. Core allows
these authenticated requests when `AgentTransport__AllowInsecure=true`, which
is the default. HTTP does not protect response integrity or traffic
confidentiality, so use it only on a trusted private network or VPN. Set the
option to `false` to require HTTPS for every regular Platform Agent and inbound
Build Pool Agent.

Citadel permits only one Platform record for each Docker daemon. Creating or
editing a regular Agent platform is rejected when that Docker engine is already
registered through Local, regular Agent, or Edge Agent access. Keep one
connection method for each Docker engine.

## Requirements

Before installing the Agent, confirm that the Docker host has:

* Docker installed and running.
* Access to `/var/run/docker.sock`.
* A network address reachable from Citadel Core.
* An inbound port available for the Agent. The default is `9000`.

The Citadel Agent version should match the Citadel Core version. Citadel generates the installation command using the expected image tag.

## Builds

Regular Agent platforms can run Citadel build projects.

For builds, Citadel Core resolves the configured Git repository and branch, packages the selected build context, and sends that archive to the Agent. The Agent streams the archive to its local Docker daemon and pushes the configured image tags to the selected registry.

Keep build contexts small. Agent build context archives must fit within the current `16 MB` message envelope. Use `.dockerignore` in the context directory to exclude `.git`, dependency folders, build outputs, and logs.

The Agent version should be updated together with Citadel Core when build protocol fields change.

## Self-Managed Build Pool Agent

A self-managed Build Pool uses the same Citadel Agent runtime, but it is dedicated to builds instead of being configured as a general Docker platform.

Use this when you want builds to run on a separate builder host or in a separate local Agent container.

### Inbound build Agent

Use inbound mode when Citadel Core can reach the builder host over the network.

Start a dedicated build Agent with its own container name and host port:

```bash
docker run -d \
  --name citadel-agent-build \
  --restart=always \
  -p 9001:9000 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HUB_PUBLIC_KEY="..." \
  ghcr.io/citadel-p/citadel.agent:1.2.3
```

Then create a Build Pool in Citadel:

```text
Build Pools -> Add Build Pool
Provider: Self-managed VM / Static VM
Connection mode: Inbound Agent endpoint
Endpoint: http://<builder-host>:9001
```

If Citadel Core is running in Docker on the same host as the build Agent, use:

```text
http://host.docker.internal:9001
```

### Edge build Agent

Use edge mode when the builder host must not expose an inbound port to Citadel Core.

Create the Build Pool in Citadel first:

```text
Build Pools -> Add Build Pool
Provider: Self-managed VM / Static VM
Connection mode: Edge Agent
```

Save the pool, then use the pool's **Edge Agent enrollment** section to generate the Docker command. Run that generated command on the builder host. It will look similar to:

```bash
docker run -d \
  --name edge-build-agent \
  --restart=always \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v edge_build_agent_data:/app/data \
  -e CITADEL_AGENT_MODE="edge" \
  -e CITADEL_EDGE_AGENT_PROFILE="edge-build-agent" \
  -e CITADEL_CORE_URL="http://<citadel-core-host>:8001" \
  -e CITADEL_EDGE_ENROLLMENT_TOKEN="..." \
  -e CITADEL_EDGE_AGENT_KEY_PATH="/app/data/edge-build-agent.key" \
  -e CITADEL_EDGE_IDENTITY_PATH="/app/data/edge-build-agent.identity.json" \
  ghcr.io/citadel-p/citadel.agent:1.2.3
```

The Edge build Agent is scoped directly to the Build Pool. It is not an Edge Agent platform, and it does not require a Platform resource to exist.

The generated command is authoritative. It includes the correct container name, data volume, token, identity path, and `CITADEL_CORE_URL`.

For local Docker testing, `CITADEL_CORE_URL` should point to the Edge Agent gRPC endpoint, usually port `8001`. Port `8000` is the normal Citadel HTTP API and UI endpoint.

Before selecting the pool in a build project, click **Test** on the Build Pool. A ready result confirms the Agent is connected, advertises build capabilities, and can reach Docker.

## Create the platform

In Citadel, open:

```text
Platforms → Add platform
```

Configure the platform:

* **Connection:** `Agent`
* **Name:** A clear name identifying the Docker host
* **Agent address:** The URL Citadel Core will use to reach the Agent
* **Tags:** Optional tags for filtering and grouping resources

Example Agent address for the default setup:

```text
http://docker-host.internal.example:9000
```

When `Agent` is selected, Citadel displays an **Agent setup** section containing:

* The Citadel Core public key
* A generated Docker installation command
* The expected Agent image version

Run the generated installation command on the Docker host, then save the platform.

> The image and public key shown by Citadel are authoritative.
> The examples below are illustrative.

If Core is configured to require secure Agent transport, Citadel presents a
TLS command template instead. Replace its certificate path placeholders before
running it because Citadel cannot know the TLS file paths on the remote host.

## Install with Docker

Copy the generated Docker command from Citadel and run it on the Docker host.

It will look similar to:

```bash
docker run -d \
  --name citadel-agent \
  --restart=always \
  --label com.citadel.system=true \
  --label com.citadel.system-role=agent \
  -p 9000:9000 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /:/host:ro \
  -e HUB_PUBLIC_KEY="..." \
  ghcr.io/citadel-p/citadel.agent:1.2.3
```

Regular Agent mode is the default. Do not set:

```text
CITADEL_AGENT_MODE=edge
```

That setting is reserved for Edge Agents.

After the container starts, return to Citadel and save the platform. Citadel will validate the connection and Agent authentication.

### Restrict the published port

The following option publishes port `9000` on every host interface:

```bash
-p 9000:9000
```

When possible, bind the Agent only to the interface reachable from Citadel Core:

```bash
-p 192.168.1.25:9000:9000
```

You can also restrict access using the host firewall so that only Citadel Core can connect to the Agent port.

## Install with Docker Compose

The Agent can also be managed using Docker Compose.

Use the public key and image tag displayed by Citadel:

```yaml
services:
  citadel-agent:
    image: ghcr.io/citadel-p/citadel.agent:1.2.3
    container_name: citadel-agent
    restart: always
    labels:
      com.citadel.system: "true"
      com.citadel.system-role: "agent"
    ports:
      - "9000:9000"
    environment:
      HUB_PUBLIC_KEY: "..."
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - /:/host:ro
```

The read-only `/host` bind lets the Agent report the capacity of the filesystem
containing Docker's configured data root. It exposes host paths to the trusted
Agent process but does not allow writes through the mount. Removing it disables
only disk metrics. Upgrade and recreate older Agent containers to add the mount.
For ZFS, the metric represents the mounted dataset or filesystem, not total pool
allocation.

Start the Agent:

```bash
docker compose up -d
```

Citadel displays the Agent container as **System** on the remote Platform. It
remains available for inspection, logs, terminal access, and resource
statistics, but Citadel blocks lifecycle and delete actions that could sever
the Platform connection. Manage the Agent from its Docker host and recreate
older Agent containers once to apply the labels.

Then configure the address that Citadel Core can use to reach it:

```text
http://docker-host.internal.example:9000
```

## Configure the Agent address

The Agent address must be reachable from **Citadel Core**, not only from your browser or workstation.

Depending on your network, you can use:

* A private LAN IP address
* A DNS hostname
* A Docker network service name
* A direct HTTP or HTTPS Agent address

Examples:

```text
http://192.168.1.25:9000
```

```text
http://docker-host.internal.example:9000
```

```text
http://citadel-agent:9000
```

The Docker service name can be used when Citadel Core and the Agent are attached to the same Docker network.

When optional TLS is enabled, each hostname or IP must be covered by the Agent
certificate. Set `AgentTransport__AllowInsecure=false` on Core to reject HTTP
Agent addresses.

Avoid using `localhost` unless Citadel Core and the Agent share the same network namespace. In most containerized installations, `localhost` from Citadel Core refers to the Core container itself—not the Agent host.

## Verify the Agent

Confirm that the Agent container is running:

```bash
docker ps --filter name=citadel-agent
```

Review its logs:

```bash
docker logs citadel-agent
```

Then save the platform in Citadel. The save operation validates that:

* The Agent address is reachable.
* The Agent is running in regular mode.
* The Agent accepts requests signed by Citadel Core.
* The Agent can communicate with the Docker daemon.

## Rotate the signing key

Citadel Core uses its signing key to authenticate requests sent to regular Agents.

Based on the shared Core signing-key model, rotation affects **every regular Agent connected to that Citadel Core instance**, not only the platform from which the action is initiated.

The public key stored in `HUB_PUBLIC_KEY` is not confidential. Rotate the key when:

* The Citadel Core private key may have been compromised.
* The private key was copied or exposed accidentally.
* Your security policy requires periodic key rotation.
* You are intentionally replacing the Core signing identity.

### Rotation procedure

Open an Agent platform form and select:

```text
Agent setup → Rotate key
```

Confirm the action by typing:

```text
rotate
```

After rotation:

* Citadel Core immediately signs requests using the new private key.
* Existing regular Agents still contain the previous public key.
* Those Agents reject new Core requests until their configuration is updated.
* Edge Agents are not affected by this signing-key rotation.

> Rotation causes an immediate interruption for regular Agents. Perform it during a maintenance window and update all Agents promptly.

Copy the newly generated Docker command or public key and update every regular Agent connected to this Core instance.

### Docker installation

Remove the existing Agent container:

```bash
docker rm -f citadel-agent
```

Run the new installation command generated by Citadel.

### Docker Compose installation

Update `HUB_PUBLIC_KEY` in the Compose file, then recreate the container:

```bash
docker compose up -d
```

Confirm that the Agent reconnects successfully by reviewing its logs and validating the platform in Citadel.

## Update the Agent image

After updating Citadel Core, update regular Agents to the matching image version displayed in the platform setup instructions.

### Docker installation

Remove the existing container:

```bash
docker rm -f citadel-agent
```

Then run the updated Docker command generated by Citadel.

### Docker Compose installation

Update the image tag in the Compose file, then run:

```bash
docker compose pull
docker compose up -d
```

Verify the running image:

```bash
docker inspect citadel-agent --format '{{.Config.Image}}'
```

## Troubleshooting

### Connection refused

Citadel Core cannot establish a network connection to the Agent.

Check that:

* The Agent container is running.
* Port `9000` is published or otherwise reachable.
* The Agent address contains the correct hostname and port.
* The host firewall allows connections from Citadel Core.
* Citadel Core can route traffic to the Docker host.

Inspect the container:

```bash
docker ps --filter name=citadel-agent
docker logs citadel-agent
```

### Request is unauthenticated

The Agent cannot validate the request signature from Citadel Core.

This usually means:

* `HUB_PUBLIC_KEY` contains the wrong public key.
* The Core signing key was rotated.
* The Agent was configured using setup information from another Citadel Core instance.
* The public key value was truncated or incorrectly escaped.

Copy the current setup command from Citadel, update the Agent configuration, and recreate the container.

### Platform save fails

Citadel validates the Agent while saving an Agent platform.

Confirm that:

* The Agent address is reachable from Citadel Core.
* The Agent container is running.
* The Agent is running in regular mode.
* The Agent and Core versions are compatible.
* `HUB_PUBLIC_KEY` matches the current Citadel Core signing key.

### Docker operations fail

Confirm that the Docker socket is mounted:

```text
/var/run/docker.sock:/var/run/docker.sock
```

Also confirm that:

* Docker is running on the host.
* The socket exists at `/var/run/docker.sock`.
* The Agent process has permission to access the socket.

### Agent starts in Edge mode

Remove the following environment variable from the container configuration:

```text
CITADEL_AGENT_MODE=edge
```

Regular Agent mode is used automatically when no Agent mode is specified.

### Agent version does not match Core

Copy the latest installation command from the platform setup section and recreate the Agent using the generated image tag.

## Security recommendations

* Protect the Citadel Core data directory. It contains the private signing key.
* Do not copy the private signing key to an Agent.
* The public key is safe to distribute, but its integrity must be preserved.
* Treat access to `/var/run/docker.sock` as privileged host access.
* Do not run untrusted workloads inside the Agent container.
* Restrict the Agent port so that only Citadel Core can reach it.
* Do not expose the Agent directly to the public internet.
* Use a private network, VPN, or TLS when traffic crosses an untrusted network.
* Keep Citadel Core and Agent versions aligned.
* Rotate the signing key immediately if the Core private key may have been compromised.

For the supported direct TLS setup and private-CA trust, see
`docs/user/tls-and-secure-agent-transport.md`.
