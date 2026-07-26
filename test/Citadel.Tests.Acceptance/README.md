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

The RustFS compatibility suite imports the supplied Core image into an isolated
Docker-in-Docker platform and runs pinned RustFS in the same daemon. It
initializes and validates an S3-compatible repository, rejects invalid
credentials and missing buckets, backs up known volume bytes, restores a
selected snapshot, compares byte count and SHA-256, applies retention and
pruning, and verifies that an object outside the repository prefix survives.

The Keycloak compatibility suite starts a pinned Keycloak image with a
deterministic realm import and a real candidate Core process. It verifies
discovery metadata, authorization-code login startup with PKCE, absence of the
client secret from the browser redirect, and rejection of a disabled provider.
The selected real-browser login, refresh, policy-rejection, and logout journey
lives in `Citadel.Tests.E2E`.

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

The candidate-image compatibility tests are skipped by this command unless
their `CITADEL_ACCEPTANCE_*_IMAGE` variables are set. The dedicated commands
below show the required image builds and environment variables.

Run both candidate-image compatibility suites with one command:

```powershell
.\test\Citadel.Tests.Acceptance\run-image-compatibility.ps1
```

The runner builds `citadel-core:acceptance` from this repository and
`citadel-agent:acceptance` from the sibling `Citadel.Agent` repository. Supply
`-AgentRepository` when the Agent checkout is elsewhere. To reuse images that
are already built:

```powershell
.\test\Citadel.Tests.Acceptance\run-image-compatibility.ps1 -SkipBuild
```

The runner requires both suites to execute, fails on missing images, and
restores the process environment variables after completion.

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
docker build -t citadel-agent:acceptance ..\Citadel.Agent

$env:CITADEL_ACCEPTANCE_REQUIRE_CANDIDATE_IMAGES = "true"
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

Run only the Keycloak OIDC protocol compatibility suite:

```powershell
dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release -- -class Tests.Acceptance.Compatibility.KeycloakOidcCompatibilityTests
```

Run the real Core image and RustFS backup compatibility suite:

```powershell
docker build -f src/Citadel.WebApi/Dockerfile -t citadel-core:acceptance .
$env:CITADEL_ACCEPTANCE_REQUIRE_CANDIDATE_IMAGES = "true"
$env:CITADEL_ACCEPTANCE_CORE_IMAGE = "citadel-core:acceptance"

dotnet run --project test/Citadel.Tests.Acceptance/Citadel.Tests.Acceptance.csproj -c Release -- -class Tests.Acceptance.Compatibility.RustFsBackupCompatibilityTests
```

The suite uses the candidate Core image as both the application and the
platform backup-helper image. RustFS is pinned by digest, and the bucket,
PostgreSQL database, Docker daemon, volumes, and object data are disposable.
The test does not create backup volumes or buckets in the host Docker daemon.

The PostgreSQL container and databases are disposable. Do not modify
`script0001.sql` after release. The acceptance suite pins its normalized SHA-256
hash so accidental edits to the baseline fail explicitly.

Once a stable Citadel image exists, add it as the previous-version baseline and
run the candidate image against preserved PostgreSQL and Citadel data volumes.
The SQL fixture remains useful for deterministic migration regression coverage.
