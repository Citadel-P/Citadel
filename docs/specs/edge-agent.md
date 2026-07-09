# Citadel Edge Agent - Implementation Spec

## Goal

Implement Citadel Edge Agent mode.

In Edge Agent mode, the Citadel Agent does not expose an inbound management port. Instead, the agent opens and maintains an outbound bidirectional gRPC connection to Citadel Core. Citadel Core routes Docker/platform operations over that active connection.

This allows Citadel to manage Docker platforms behind NAT, firewalls, CGNAT, private customer networks, or remote sites where exposing the agent on `:9000` is undesirable.

Existing connector modes must continue to work:

```text
PlatformConnectorType:
- Local
- Agent
- EdgeAgent
```

## Current Architecture Fit

Citadel can evolve toward this model without rewriting application handlers because Docker operations already go through connector interfaces selected by `PlatformConnectorType`.

Current flow:

```text
Application handler
  -> IConnectorFactory<T>.GetConnector(platform.ConnectorType)
  -> Agent*Connector
  -> gRPC client to Platforms.Address
  -> Citadel Agent :9000
  -> Docker host
```

Edge flow:

```text
Application handler
  -> IConnectorFactory<T>.GetConnector(PlatformConnectorType.EdgeAgent)
  -> Edge*Connector
  -> IEdgeAgentCommandRouter
  -> active EdgeAgentSession
  -> connected agent
  -> Docker host
```

The main architecture change is not in the application features. It is behind the connector boundary.

Important current constraints:

- Current `Agent` connectors dial `Platforms.Address` directly.
- Current hub-to-agent gRPC requests are signed by Core and verified by Agent.
- Responses and response streams are not signed.
- Health checks are active Core-to-agent polls.
- Streaming operations depend on Core-initiated gRPC calls.

Edge mode must introduce Core-side agent authentication and an active session registry. It should not reuse the current inbound request-signing model blindly.

## Implemented Scope

The implementation builds the reverse gRPC foundation and routes the current Docker connector surface through the active Edge Agent session.

Implement:

```text
Core:
- PlatformConnectorType.EdgeAgent
- Edge enrollment token generation
- Edge agent binding persistence
- Core-hosted EdgeAgentService gRPC stream
- Edge session registry
- Edge command router
- Edge heartbeat/status tracking
- Edge platform health integration
- Minimal API endpoints for enrollment/status/revoke
- UI support for creating an Edge Agent platform and copying enrollment instructions
- Edge connectors for platform, container, image, volume, network, stack, and deployment operations
- Transport hardening:
  - protocol version validation
  - capability validation
  - bounded command/session queues
  - maximum envelope payload size
  - command timeout cancellation sent back to the agent
  - duplicate session replacement
  - command lifecycle logging without payload logging

Agent:
- CITADEL_AGENT_MODE=edge
- Outbound connection to Core
- Agent key generation and persistence
- First-time enrollment
- Reconnect authentication using agent private key
- Heartbeat
- Command dispatching
- HTTP/2 keepalive pings
- exponential reconnect backoff with jitter
- command timeout enforcement
- bounded command input queues
- maximum envelope payload size
```

The Edge Agent now supports the main existing Docker operations routed through Citadel connector interfaces.

## Explicit Non-Goals For This PR

Still not implemented:

```text
- transparent reverse HTTP/2 tunnel
- polling work queue
- SignalR agent client
- multi-Core distributed routing
- offline command queueing
- Kubernetes edge mode
- agent-to-agent mesh
- key rotation
- conversion from existing Agent platform to EdgeAgent
- running inbound and edge mode at the same time
```

Edge mode and inbound agent mode are mutually exclusive in this MVP.

## Recommended Architecture

Use a long-lived bidirectional gRPC control channel.

```text
Citadel Agent
  -> EdgeAgentService.Connect()
  -> Citadel Core

Citadel Core
  -> command envelope over active session
  -> Citadel Agent

Citadel Agent
  -> command output/completion over same session
  -> Citadel Core
```

Use an explicit command bus, not a transparent tunnel.

Why this is the right MVP direction:

- Citadel already has a connector boundary that can absorb the transport change.
- Citadel has interactive streams, so a polling work queue would be a poor primary transport.
- A transparent reverse HTTP/2 tunnel is harder to debug and more complex than a typed command bus.
- SignalR is better kept for browser/UI streams, not infrastructure control.
- gRPC/protobuf fits the existing agent contract and source-generated message model.

## Domain Model

### Platform Connector Type

Add:

```csharp
public enum PlatformConnectorType
{
    Unknown,
    Local,
    Agent,
    EdgeAgent
}
```

For MVP, if `Platforms.Address` is required, store:

```text
edge://{platformId}
```

Do not expose this as a network address in the UI.

Long term, platform connection settings can be split into a dedicated table/value object. Do not do that refactor in this PR unless the current model makes `EdgeAgent` impossible.

### Platform Creation

The current `CreatePlatform` path calls `IPlatformConnector.GetPlatformAsync` before saving a platform. Edge platforms cannot do this before enrollment.

For `ConnectorType = EdgeAgent`, create a pending platform without dialing an agent:

```text
ConnectorType = EdgeAgent
Address = edge://{platformId}
Status = Offline or Pending equivalent in the edge status projection
```

After enrollment and heartbeat, platform metadata such as Docker version, agent version, CPU count, memory, images, containers, and networks can be synchronized.

## Persistence

Add separate persistence for enrollments and bindings.

Follow the project migration guidelines:

- model schema changes in `src/Citadel.Infrastructure.Migrations\EntityFramework\ApplicationDbContext.cs`
- generate EF migrations under `src/Citadel.Infrastructure.Migrations\Migrations`
- regenerate SQL scripts from the migration project
- do not hand-write migration SQL scripts

### EdgeAgentEnrollments

Create a table:

```text
EdgeAgentEnrollments
- Id uuid primary key
- PlatformId uuid not null
- TokenHash text not null
- ExpiresAtUtc timestamptz not null
- UsedAtUtc timestamptz null
- RevokedAtUtc timestamptz null
- CreatedByActorId uuid not null
- CreatedAtUtc timestamptz not null
```

Rules:

```text
- Token is displayed once.
- Store only a hash.
- Token is single-use.
- Token is scoped to one platform.
- Token must expire.
- Reused, expired, revoked, or unknown tokens must be rejected.
```

Recommended token format:

```text
Random 32+ bytes, base64url encoded.
```

### EdgeAgentBindings

Create a table:

```text
EdgeAgentBindings
- Id uuid primary key
- PlatformId uuid not null unique
- AgentId uuid not null
- AgentPublicKey text not null
- AgentFingerprint text not null
- ConnectionStatus text not null
- LastConnectedAtUtc timestamptz null
- LastDisconnectedAtUtc timestamptz null
- LastHeartbeatAtUtc timestamptz null
- LastSeenVersion text null
- LastSeenHostname text null
- CapabilitiesJson jsonb null
- ProtocolVersion int not null default 1
- RevokedAtUtc timestamptz null
- CreatedAtUtc timestamptz not null
- UpdatedAtUtc timestamptz not null
```

Connection status values persisted on binding:

```text
Offline
Connected
Revoked
```

`PendingEnrollment` should be derived by the application/UI when:

```text
platform.ConnectorType == EdgeAgent
AND no non-revoked binding exists
AND an active enrollment exists or the platform has not enrolled yet
```

Do not create a binding row with null `AgentId`, public key, or fingerprint only to represent pending enrollment.

Fingerprint definition:

```text
SHA-256 hash of the encoded Ed25519 public key.
```

UI can display a shortened version:

```text
SHA256:ab12cd34...7890
```

## Enrollment Flow

### Core-Side Flow

Admin creates an Edge Agent platform.

```text
ConnectorType = EdgeAgent
Address = edge://{platformId}
ConnectionStatus projection = PendingEnrollment
```

Admin generates an enrollment token.

Core stores:

```text
TokenHash
PlatformId
ExpiresAtUtc
CreatedByActorId
CreatedAtUtc
```

Core returns the plaintext token once.

### Agent First Start

Agent is configured with:

```bash
CITADEL_AGENT_MODE=edge
CITADEL_CORE_URL=https://citadel.example.com
CITADEL_EDGE_ENROLLMENT_TOKEN=...
CITADEL_EDGE_AGENT_KEY_PATH=/data/edge-agent.key
CITADEL_EDGE_IDENTITY_PATH=/data/edge-agent.identity.json
```

Startup behavior:

```text
1. Agent detects CITADEL_AGENT_MODE=edge.
2. Agent disables public inbound management gRPC listener.
3. Agent loads existing private key or generates a new Ed25519 key pair.
4. Agent connects outbound to Citadel Core.
5. Agent sends enrollment request:
   - enrollment token
   - public key
   - hostname
   - agent version
   - capabilities
   - protocol version
6. Core validates the enrollment token.
7. Core stores the agent public key and fingerprint.
8. Core marks the token as used.
9. Core returns:
   - PlatformId
   - AgentId
   - SessionId
10. Agent persists assigned identity to CITADEL_EDGE_IDENTITY_PATH.
11. Agent must not persist the enrollment token.
```

Do not require `CITADEL_EDGE_CONNECT_AS` in the normal MVP flow.

### Reconnect Flow

After enrollment, the agent reconnects without using the enrollment token.

```text
1. Agent loads:
   - private key
   - PlatformId
   - AgentId
2. Agent connects to Core.
3. Agent sends hello with PlatformId, AgentId, fingerprint, version, capabilities.
4. Core checks the binding exists and is not revoked.
5. Core sends nonce challenge.
6. Agent signs the challenge using its private key.
7. Core verifies signature against stored agent public key.
8. Core accepts session.
9. Core marks platform connection status as Connected.
```

A revoked agent key must not reconnect without a new enrollment.

## gRPC Contract

Add a new proto file under `Citadel.Contracts`:

```text
edge_agent_service.proto
```

Because `Citadel.Contracts` is a git submodule used by both Citadel Core and Citadel Agent, proto changes must be checked in for both repositories.

### Service

```proto
service EdgeAgentService {
  rpc Connect(stream AgentEnvelope) returns (stream CoreEnvelope);
}
```

### Command Kind

Use enum-based routing. Do not use string `Service` / `Method` routing.

```proto
enum EdgeCommandKind {
  EDGE_COMMAND_KIND_UNSPECIFIED = 0;

  PLATFORM_CHECK_HEALTH = 1;

  CONTAINER_LIST = 10;
  CONTAINER_LOGS_STREAM = 11;
}
```

Only implement the listed command kinds in this MVP.

### AgentEnvelope

```proto
message AgentEnvelope {
  string envelope_id = 1;
  string session_id = 2;
  string command_id = 3;

  oneof body {
    AgentHello hello = 10;
    EnrollmentRequest enrollment_request = 11;
    AuthChallengeResponse auth_challenge_response = 12;
    AgentHeartbeat heartbeat = 13;

    CommandOutput command_output = 20;
    CommandCompleted command_completed = 21;
    CommandFailed command_failed = 22;
  }
}
```

### CoreEnvelope

```proto
message CoreEnvelope {
  string envelope_id = 1;
  string session_id = 2;
  string command_id = 3;

  oneof body {
    AuthChallenge auth_challenge = 10;
    SessionAccepted session_accepted = 11;
    SessionRejected session_rejected = 12;

    EdgeCommand command = 20;
    StreamInput stream_input = 21;
    CancelCommand cancel_command = 22;

    Disconnect disconnect = 30;
  }
}
```

### EdgeCommand

```proto
message EdgeCommand {
  string command_id = 1;
  string platform_id = 2;
  EdgeCommandKind kind = 3;
  bytes payload = 4;
  int32 payload_schema_version = 5;
  int32 timeout_ms = 6;
  string correlation_id = 7;
  bool expects_stream = 8;
}
```

Use `timeout_ms`, not only `deadline_utc`.

Reason:

```text
- DeadlineUtc depends on synchronized clocks.
- TimeoutMs can be enforced relative to command receipt.
```

Existing protobuf request/response messages from the current inbound Agent path should be reused where possible.

## Core Components

### EdgeAgentService

Core-hosted gRPC service.

Responsibilities:

```text
- Accept outbound agent Connect streams
- Process enrollment
- Process reconnect authentication
- Register active sessions
- Read AgentEnvelope messages
- Write CoreEnvelope messages
- Handle disconnects
- Mark session offline on stream termination
```

Important rule:

Do not write to the gRPC response stream directly from random request threads.

Use a bounded outbound channel per session:

```text
CommandRouter
  -> session.OutboundChannel.Writer.WriteAsync(...)

EdgeAgentService writer loop
  -> reads session.OutboundChannel
  -> writes CoreEnvelope to gRPC stream
```

### EdgeAgentSessionRegistry

Create a session registry.

Responsibilities:

```text
- Track active sessions by PlatformId
- Track active sessions by AgentId
- Enforce one active session per platform
- Replace older session when newer authenticated session connects
- Cancel in-flight commands when a session is replaced
- Remove sessions on disconnect
- Expose TryGetSession(platformId)
```

Session object should contain:

```text
- SessionId
- PlatformId
- AgentId
- AgentFingerprint
- ConnectedAtUtc
- LastHeartbeatAtUtc
- OutboundChannel
- PendingCommands
- SessionCancellationTokenSource
```

### EdgeAgentCommandRouter

Create an interface:

```csharp
public interface IEdgeAgentCommandRouter
{
    Task<byte[]> SendUnaryAsync(
        Guid platformId,
        EdgeCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);

    IAsyncEnumerable<byte[]> SendServerStreamAsync(
        Guid platformId,
        EdgeCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken);
}
```

Behavior:

```text
- Find active session by PlatformId.
- If no active session exists, return PlatformUnreachable.
- Create CommandId.
- Register pending command before sending.
- Send EdgeCommand to session outbound channel.
- Await command output/completion/failure.
- Enforce timeout.
- Propagate cancellation.
- If session disconnects, fail pending commands with stream interrupted/platform unreachable.
- Remove pending command after completion/failure/cancellation.
```

Prefer result-based failures consistent with the application layer instead of using exceptions for expected states such as no active session.

### EdgeAgentEnrollmentService

Responsibilities:

```text
- Generate enrollment tokens
- Hash tokens
- Validate enrollment requests
- Mark token used
- Create or update EdgeAgentBinding
- Reject expired/reused/revoked tokens
```

### EdgeAgentHealthService

Responsibilities:

```text
- Read current edge connection status
- Read latest heartbeat
- Expose edge platform status to health monitor/UI
```

Do not treat "agent connected" as equivalent to "Docker healthy."

Track separately:

```text
Edge connection:
- PendingEnrollment
- Offline
- Connected
- Revoked

Docker/platform health:
- Unknown
- Healthy
- Degraded
- Unreachable
```

Heartbeat should include lightweight Docker health info if available:

```text
- DockerReachable
- DockerVersion
- Hostname
- AgentVersion
- Capabilities
```

## Agent Components

### EdgeAgentHostedService

Runs only when:

```bash
CITADEL_AGENT_MODE=edge
```

Responsibilities:

```text
- Load/generate Ed25519 key pair
- Load persisted edge identity if available
- Connect to CITADEL_CORE_URL
- Enroll if enrollment token exists and identity does not exist
- Reconnect using challenge signature if identity exists
- Maintain read/write loops
- Send heartbeat
- Reconnect with exponential backoff and jitter
- Stop cleanly on shutdown
```

Recommended reconnect behavior:

```text
Initial delay: 1 second
Max delay: 60 seconds
Use jitter
Reset backoff after successful connection
```

### EdgeIdentityStore

Persists:

```json
{
  "platformId": "...",
  "agentId": "...",
  "agentFingerprint": "...",
  "coreUrl": "https://citadel.example.com"
}
```

Private key is stored separately at:

```bash
CITADEL_EDGE_AGENT_KEY_PATH=/data/edge-agent.key
```

### EdgeCommandDispatcher

Responsibilities:

```text
- Receive EdgeCommand
- Validate command kind is supported
- Reject unsupported command kinds
- Deserialize payload
- Dispatch to existing agent Docker service layer
- Return command output/completion/failure
- Enforce timeout
- Observe cancellation
```

Supported command groups:

```text
- platform health/info/stats/events
- container list/logs/inspect/create/start/stop/pause/unpause/restart/delete/stats/exec
- image get/list/inspect/delete/history/exposed ports/distribution inspect/pull
- volume list/inspect/create/delete
- network list/inspect/create/delete
- stack apply
- deployment apply
```

### EdgeCommandExecutionRegistry

Responsibilities:

```text
- Track running commands by CommandId
- Cancel command on CancelCommand
- Cancel all commands on disconnect/shutdown
- Support future streaming/duplex commands
```

## Connector Changes

Add edge connector implementations only for the MVP slice.

### EdgePlatformConnector

Implement:

```text
CheckHealthAsync
```

Behavior:

```text
- If no active session exists, return unreachable/offline.
- If active session exists and latest heartbeat says DockerReachable=true, return healthy.
- If active session exists but DockerReachable=false, return degraded/unreachable.
- Do not dial Platforms.Address.
```

### EdgeContainerConnector

Implement:

```text
ListContainersAsync
StreamContainerLogsAsync
```

Behavior:

```text
- Serialize existing agent protobuf request where possible.
- Send command through IEdgeAgentCommandRouter.
- Deserialize existing agent protobuf response where possible.
- Reuse current Agent connector mapping helpers where possible.
```

Avoid duplicating domain/protobuf mapping logic.

If existing `AgentContainerConnector` contains reusable mapping logic, extract shared helpers instead of copying large blocks.

## API / UI Requirements

### API

Add minimal endpoints consistent with existing Citadel API style.

Recommended endpoints:

```http
POST /api/v1/platforms/{platformId}/edge/enrollments
GET  /api/v1/platforms/{platformId}/edge/status
POST /api/v1/platforms/{platformId}/edge/revoke
```

Endpoint classes should remain thin and delegate business logic to application commands/queries.

Add request/response models to the source-generated JSON context if needed.

#### Generate Enrollment Token

```http
POST /api/v1/platforms/{platformId}/edge/enrollments
```

Rules:

```text
- Platform must exist.
- Platform ConnectorType must be EdgeAgent.
- Caller must be authorized to update/manage platform.
- Token is returned once.
- Token expiration should default to 24 hours for MVP.
```

Response:

```json
{
  "token": "...",
  "expiresAtUtc": "...",
  "instructions": {
    "coreUrl": "https://...",
    "environment": {
      "CITADEL_AGENT_MODE": "edge",
      "CITADEL_CORE_URL": "https://...",
      "CITADEL_EDGE_ENROLLMENT_TOKEN": "...",
      "CITADEL_EDGE_AGENT_KEY_PATH": "/data/edge-agent.key",
      "CITADEL_EDGE_IDENTITY_PATH": "/data/edge-agent.identity.json"
    }
  }
}
```

#### Edge Status

```http
GET /api/v1/platforms/{platformId}/edge/status
```

Response:

```json
{
  "connectionStatus": "Connected",
  "lastConnectedAtUtc": "...",
  "lastDisconnectedAtUtc": "...",
  "lastHeartbeatAtUtc": "...",
  "lastSeenVersion": "...",
  "lastSeenHostname": "...",
  "agentFingerprint": "SHA256:ab12cd34...7890",
  "protocolVersion": 1,
  "revokedAtUtc": null
}
```

#### Revoke

```http
POST /api/v1/platforms/{platformId}/edge/revoke
```

Behavior:

```text
- Mark binding revoked.
- Terminate active session.
- Cancel in-flight commands.
- Reject future reconnects from the revoked key.
- Add activity event.
```

### UI

Platform add/edit form:

```text
Connector Type:
- Local
- Agent
- Edge Agent
```

When `Edge Agent` is selected:

```text
- Hide inbound agent address field.
- Show explanation that the agent connects outbound to Core.
- Allow creating enrollment token after platform creation.
```

Platform detail page should show:

```text
- Edge connection status
- Last connected time
- Last heartbeat time
- Agent version
- Hostname
- Agent fingerprint
- Token expiration if an active enrollment exists
- Reconnect/deployment instructions
- Revoke button
```

## Security Requirements

### Transport

Production Edge Agent mode requires HTTPS.

Self-signed/dev mode may be allowed only behind an explicit development-only flag.

Do not silently disable TLS validation in production.

### Agent Identity

```text
- Agent generates Ed25519 key pair.
- Agent persists private key locally.
- Core stores public key.
- Core pins public key fingerprint to the platform.
```

### Enrollment Token

```text
- Bootstrap credential only.
- Not a long-term credential.
- Single-use.
- Hashed at rest.
- Expires.
- Scoped to one platform.
- Never logged.
```

### Reconnect Auth

```text
- Core sends nonce challenge.
- Challenge includes nonce and timestamp.
- Agent signs challenge.
- Core verifies signature with stored public key.
```

### Authorization

Core remains the policy authority.

Application handlers must authorize before connector calls.

Agent is not a policy engine in MVP.

Do not rely on agent-side authorization for user permissions.

### Payload Logging

Do not log:

```text
- command payloads
- registry credentials
- env files
- stack secrets
- generated secret files
- enrollment tokens
- private keys
```

Existing redaction layer must still apply to command output/progress logs.

## Operational Behavior

### Online/Offline

```text
Connected session = edge connection online.
Missed heartbeats = edge connection offline.
DockerReachable=false = platform degraded/unreachable, not necessarily edge offline.
```

Recommended heartbeat interval:

```text
15 seconds
```

Recommended offline threshold:

```text
3 missed heartbeats
```

Make these configurable if Citadel already has similar options.

### Duplicate Connections

Default behavior:

```text
- One active session per platform.
- New authenticated session replaces old session.
- Old session is disconnected.
- In-flight commands on old session are canceled.
```

### Core Restart

```text
- In-memory sessions are lost.
- Agents reconnect automatically.
- Edge platforms remain Offline until their agents reconnect.
- Re-enrollment is not required.
```

### Agent Restart

```text
- Agent reuses persisted private key.
- Agent reuses persisted identity.
- Core recognizes agent and accepts reconnect after challenge signature.
```

### Command Failure Modes

```text
No active session:
- Return PlatformUnreachable.

Command timeout:
- Core sends CancelCommand.
- Return timeout.

Agent disconnects mid-stream:
- Return stream interrupted.
- Fail pending commands.
- Mark session disconnected.
```

## Backpressure And Limits

Use bounded channels.

Current conservative limits:

```text
- Max concurrent commands per edge session
- Max active streams per edge session
- Max envelope size
- Max command payload size
- Max outbound queue size
```

Defaults:

```text
MaxConcurrentCommandsPerSession = 16
MaxActiveStreamsPerSession = 8
MaxOutboundQueueSize = 256
MaxEnvelopePayloadBytes = 16 MiB
```

If a queue is full, fail fast with a clear error rather than allowing unbounded memory growth.

Command timeouts are enforced by Core and also sent to the Agent as `CancelCommand` so running Docker work can be canceled when possible. Agent-side command execution also respects the `timeout_ms` value from Core.

The Agent uses HTTP/2 keepalive pings to detect half-open connections:

```text
KeepAlivePingDelay = 30 seconds
KeepAlivePingTimeout = 10 seconds
ReconnectInitialDelay = 1 second
ReconnectMaxDelay = 60 seconds
Reconnect uses jitter
```

## Activity Events

Add activity events:

```text
- edge agent enrollment token created
- edge agent enrolled
- edge agent connected
- edge agent disconnected
- edge agent revoked
```

Do not create noisy heartbeat activity events.

## Alerts

For MVP, integrate only with existing platform unreachable/degraded behavior if possible.

Add a dedicated alert later if needed:

```text
Edge agent disconnected
```

Do not create excessive alert/event noise.

## Implementation Order

### Step 1 - Models And Migrations

```text
- Add PlatformConnectorType.EdgeAgent.
- Add EdgeAgentEnrollments table.
- Add EdgeAgentBindings table.
- Add repositories/services consistent with existing persistence patterns.
```

### Step 2 - gRPC Contract

```text
- Add edge_agent_service.proto.
- Generate contracts in both Citadel Core and Citadel Agent.
- Add EdgeCommandKind enum.
- Add envelope messages.
```

### Step 3 - Core Session Infrastructure

```text
- Implement EdgeAgentService.
- Implement EdgeAgentSessionRegistry.
- Implement outbound channel per session.
- Implement heartbeat handling.
- Implement duplicate connection replacement.
```

### Step 4 - Enrollment

```text
- Implement enrollment token generation.
- Implement enrollment validation.
- Implement agent binding.
- Implement reconnect challenge verification.
- Implement revoke behavior.
```

### Step 5 - Agent Edge Client

```text
- Add CITADEL_AGENT_MODE=edge.
- Disable public inbound management gRPC listener in edge mode.
- Implement EdgeAgentHostedService.
- Implement key persistence.
- Implement identity persistence.
- Implement enrollment/reconnect.
- Implement heartbeat.
```

### Step 6 - Command Router

```text
- Implement IEdgeAgentCommandRouter.
- Support unary commands.
- Support server-streaming commands.
- Support cancellation.
- Support timeout.
- Support session disconnect failure.
```

### Step 7 - MVP Commands

```text
- Implement CONTAINER_LIST.
- Implement CONTAINER_LOGS_STREAM.
- Implement PLATFORM_CHECK_HEALTH.
```

### Step 8 - Edge Connectors

```text
- Implement EdgePlatformConnector.CheckHealthAsync.
- Implement EdgeContainerConnector.ListContainersAsync.
- Implement EdgeContainerConnector.StreamContainerLogsAsync.
- Register EdgeAgent in connector factory.
```

### Step 9 - API And UI

```text
- Add enrollment endpoint.
- Add status endpoint.
- Add revoke endpoint.
- Add platform form support for Edge Agent.
- Add platform detail Edge Agent status panel.
- Add deployment instructions using env vars.
```

### Step 10 - Tests

Add tests for:

```text
Enrollment:
- valid token enrolls successfully
- expired token rejected
- reused token rejected
- revoked token rejected
- token hash is stored, plaintext is not stored

Authentication:
- enrolled agent reconnects with valid signature
- invalid signature rejected
- revoked agent rejected

Session:
- session registered on connect
- session removed on disconnect
- duplicate connection replaces old session
- replacing session cancels in-flight commands

Command router:
- unary command succeeds
- streaming command yields multiple outputs
- no active session returns PlatformUnreachable
- timeout cancels command
- disconnect during stream returns interrupted

Security:
- enrollment token is not logged
- command payload is not logged
```

## Acceptance Criteria

The implementation is complete when:

```text
- A platform can be created with ConnectorType = EdgeAgent.
- EdgeAgent platforms do not require a visible inbound address.
- Admin can generate a single-use enrollment token.
- Agent can enroll using outbound HTTPS/gRPC only.
- Agent does not expose the inbound management gRPC port in edge mode.
- Core stores the agent public key and fingerprint.
- Agent reconnects without the enrollment token.
- Revoking the binding prevents reconnect.
- Core shows edge status: PendingEnrollment, Connected, Offline, Revoked.
- List containers works through EdgeAgent mode.
- Container logs streaming works through EdgeAgent mode.
- Existing Local connector mode still works.
- Existing inbound Agent connector mode still works.
- If no edge session exists, connector returns a clear platform unreachable error.
- If the agent disconnects mid-stream, Core returns a clear stream interrupted error.
- No enrollment token, registry credential, env file, stack secret, or command payload is logged.
```

## Important Guardrails

Do not overbuild this feature.

This PR should prove the edge transport and session model. It should not attempt to make every existing Citadel operation work over edge mode.

After this MVP is stable, expand in this order:

```text
1. Image/network/volume read operations
2. Container mutation operations
3. Image pull/delete
4. Deployment apply
5. Stack apply/rollback
6. Stack logs
7. Exec duplex streaming
8. Key rotation
9. Existing Agent -> EdgeAgent conversion
10. Multi-Core routing support
```

## Open Questions

- Should `Platforms.Address` remain required after the MVP, or should platform connection settings become a separate value object/table?
- Which reverse proxy configurations will Citadel officially support for bidirectional gRPC edge connections?
- Should Edge Agent status be pushed over SignalR to platform pages in MVP or polled from the status endpoint?

## References

- Komodo v2 outbound Periphery connection: https://komo.do/docs/releases/v2.0.0#2b-reversing-the-agent-connection
- Portainer Edge Agent overview: https://docs.portainer.io/advanced/edge-agent
- Portainer Agent edge mode README: https://github.com/portainer/agent/blob/develop/README.md#using-the-agent-in-edge-mode
