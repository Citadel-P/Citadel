param(
    [string]$DevContainer = 'citadel_devcontainer-workspace-1',
    [string]$AgentImage = 'citadel-agent-phase7:local',
    [switch]$UseEdgeAgent
)
$ErrorActionPreference = 'Stop'
$inspection = & docker inspect $DevContainer
if ($LASTEXITCODE -ne 0) { throw 'Start the Citadel development container first.' }
$workspace = ($inspection | ConvertFrom-Json)[0]
$network = @($workspace.NetworkSettings.Networks.PSObject.Properties.Name)[0]
$agentHost = $workspace.Name.TrimStart('/')
$edgeFlag = if ($UseEdgeAgent) { '1' } else { '0' }
if (-not $workspace.State.Running -or -not $network) { throw 'The development container must be running and networked.' }
& docker image inspect $AgentImage --format '{{.Id}}'
if ($LASTEXITCODE -ne 0) { throw 'Build the current Agent candidate first.' }
$testUid = (& docker exec --user vscode $DevContainer id -u).Trim()
if ($LASTEXITCODE -ne 0 -or $testUid -notmatch '^[1-9][0-9]*$') { throw 'Could not resolve the development user.' }
$postgres = 'citadel-phase7-agent-build-' + [Guid]::NewGuid().ToString('N')
try {
    & docker pull docker:27.5.1-dind
    if ($LASTEXITCODE -ne 0) { throw 'Could not obtain the isolated Docker daemon image.' }
    & docker pull registry@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373
    if ($LASTEXITCODE -ne 0) { throw 'Could not obtain the Registry test image.' }
    & docker run --detach --name $postgres --network $network --tmpfs /var/lib/postgresql --env POSTGRES_USER=citadel --env POSTGRES_PASSWORD=citadel --env POSTGRES_DB=citadel postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the isolated test database.' }
    foreach ($attempt in 1..30) { & docker exec $postgres pg_isready -U citadel -d citadel *> $null; if ($LASTEXITCODE -eq 0) { break }; Start-Sleep -Milliseconds 500 }
    & docker exec --user "${testUid}:0" --workdir /workspace/rust --env DOCKER_HOST=unix:///var/run/docker-host.sock `
        --env "CITADEL_PHASE7_DATABASE_URL=postgres://citadel:citadel@${postgres}:5432/citadel" `
        --env "CITADEL_PHASE7_AGENT_IMAGE=$AgentImage" --env "CITADEL_PHASE7_AGENT_NETWORK=$network" `
        --env "CITADEL_PHASE7_BUILD_EDGE=$edgeFlag" --env "CITADEL_PHASE7_AGENT_HOST=$agentHost" `
        $DevContainer cargo test --locked -p citadel-adapters --test build_agent_acceptance -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'Published Agent Build/push acceptance failed.' }
} finally { & docker rm --force --volumes $postgres | Out-Null }
