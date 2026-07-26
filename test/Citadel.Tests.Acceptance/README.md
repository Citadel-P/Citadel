# Citadel Product Acceptance Tests

This project contains slower product-level acceptance suites that use pinned,
disposable service containers.

The upgrade suite restores the versioned pre-release `1.0.0` database
baseline, starts the current candidate as a real Web API process, verifies
authentication and preserved state, restarts the candidate with the same
database and data directory, and performs new operations.

The control-plane recovery suite creates a custom-format `pg_dump` plus the
required file-backed security assets, destroys the source database and data
directory, restores into a clean environment, and starts the same candidate
artifact. It also proves that an archive without the secret-encryption key is
rejected before the target database is changed.

The Edge Agent recovery suite connects a protocol-level client to a real
candidate Core process. It verifies routed operation success, explicit
unavailable behavior while disconnected, authenticated reconnect, reconnect
after Core restart, and rejection after revocation. Real Core and Agent image
compatibility is covered separately so protocol diagnostics remain focused.

The Edge Agent compatibility suite starts the supplied Core and Agent images
with an isolated Docker-in-Docker daemon. It verifies enrollment, version and
protocol reporting, routed Docker operations, streamed stack deployment,
container inspection, persisted-identity reconnect, revocation, and
cross-platform command isolation.

The Forgejo compatibility suite starts a pinned, registration-disabled Forgejo
instance and a real candidate Core process. It creates a private repository,
deploys a Git-backed stack, delivers signed push webhooks, deploys a second
release, rejects an invalid signature, and verifies stack watch paths.

## Prerequisites

- Docker Desktop or another Docker Engine
- .NET 10 SDK
- Git CLI

## Run

From the repository root:

```powershell
dotnet restore test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj
dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release
```

Run only the upgrade suite:

```powershell
dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release -- -class Tests.Acceptance.Upgrades.PreReleaseUpgradeTests
```

Run only the control-plane recovery suite:

```powershell
dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release -- -class Tests.Acceptance.Recovery.ControlPlaneRecoveryTests
```

Run only the Edge Agent recovery suite:

```powershell
dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release -- -class Tests.Acceptance.Recovery.EdgeAgentRecoveryTests
```

Run the real Core and Agent image compatibility suite:

```powershell
docker build -f src/Citadel.WebApi/Dockerfile -t citadel-core:acceptance .
docker build -t citadel-agent:acceptance D:\Projects\Citadel.Agent

$env:CITADEL_ACCEPTANCE_CORE_IMAGE = "citadel-core:acceptance"
$env:CITADEL_ACCEPTANCE_AGENT_IMAGE = "citadel-agent:acceptance"
$env:CITADEL_ACCEPTANCE_AGENT_VERSION = "1.0.0" # Optional exact assertion

dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release -- -class Tests.Acceptance.Compatibility.EdgeAgentCompatibilityTests
```

The image references may also be registry references or immutable digests.
The suite creates disposable Core data, Agent identity, PostgreSQL, and
Docker data volumes; it does not deploy the test stack to the host Docker
daemon.

Run only the Forgejo Git and webhook compatibility suite:

```powershell
dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release -- -class Tests.Acceptance.Compatibility.ForgejoGitWebhookTests
```

The PostgreSQL container and databases are disposable. Do not modify
`script0001.sql` after release. The acceptance suite pins its normalized SHA-256
hash so accidental edits to the baseline fail explicitly.

Once a stable Citadel image exists, add it as the previous-version baseline and
run the candidate image against preserved PostgreSQL and Citadel data volumes.
The SQL fixture remains useful for deterministic migration regression coverage.
