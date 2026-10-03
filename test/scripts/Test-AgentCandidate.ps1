param(
    [string]$DevContainer = 'citadel_devcontainer-workspace-1',
    [string]$AgentImage = 'citadel-agent-execution:local'
)

$ErrorActionPreference = 'Stop'
$inspection = & docker inspect $DevContainer
if ($LASTEXITCODE -ne 0) { throw 'Start the Citadel development container first.' }
$workspace = ($inspection | ConvertFrom-Json)[0]
if (-not $workspace.State.Running) { throw 'The development container is not running.' }
$network = @($workspace.NetworkSettings.Networks.PSObject.Properties.Name)[0]
if (-not $network) { throw 'The development container has no Docker network.' }
& docker image inspect $AgentImage --format '{{.Id}}'
if ($LASTEXITCODE -ne 0) { throw 'Build the current Agent checkout as the requested candidate image first.' }
$testUid = (& docker exec --user vscode $DevContainer id -u).Trim()
if ($LASTEXITCODE -ne 0 -or $testUid -notmatch '^[1-9][0-9]*$') { throw 'Could not resolve the development user.' }
& docker exec --user "${testUid}:0" --workdir /workspace `
    --env DOCKER_HOST=unix:///var/run/docker-host.sock `
    --env "CITADEL_TEST_AGENT_IMAGE=$AgentImage" `
    --env "CITADEL_TEST_AGENT_NETWORK=$network" `
    $DevContainer cargo test --locked -p citadel-adapters --test agent_candidate_acceptance -- --ignored
if ($LASTEXITCODE -ne 0) { throw 'The published Agent compatibility test failed.' }
