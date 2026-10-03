param(
    [string]$WorkspaceContainer = 'citadel_devcontainer-workspace-1',
    [string]$Network = 'citadel_devcontainer_default',
    [string]$CoreImage = 'citadel-rust-execution:local',
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)

$ErrorActionPreference = 'Stop'
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$postgres = "citadel-execution-automation-pg-$suffix"
$toolsContainer = "citadel-execution-tools-$suffix"
$toolsDirectory = "/tmp/citadel-execution-tools-$suffix"

function Invoke-Docker {
    param([string[]]$Arguments)
    & docker @Arguments
    if ($LASTEXITCODE -ne 0) { throw "execution Docker command failed: $($Arguments[0])" }
}

try {
    # Extract the actual candidate tools, without building a new image or
    # mounting the development database. Missing tools fail this gate.
    Invoke-Docker @('inspect', $WorkspaceContainer) | Out-Null
    Invoke-Docker @('create', '--name', $toolsContainer, '--entrypoint', '/bin/true', $CoreImage) | Out-Null
    Invoke-Docker @('exec', '--user', '1000:0', $WorkspaceContainer, 'mkdir', '-p', $toolsDirectory)
    foreach ($tool in @('deno', 'shoutrrr')) {
        Invoke-Docker @('exec', '--user', '1000:0', '--env', 'DOCKER_HOST=unix:///var/run/docker-host.sock', $WorkspaceContainer,
            'docker', 'cp', "${toolsContainer}:/usr/local/bin/$tool", "$toolsDirectory/$tool")
    }
    Invoke-Docker @('run', '--detach', '--name', $postgres, '--network', $Network,
        '--tmpfs', '/var/lib/postgresql', '--env', 'POSTGRES_USER=citadel_execution', '--env', 'POSTGRES_PASSWORD=citadel_execution',
        '--env', 'POSTGRES_DB=citadel_execution', '--health-cmd', 'pg_isready -U citadel_execution',
        '--health-interval', '1s', '--health-timeout', '2s', '--health-retries', '30', $PostgresImage) | Out-Null
    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') { $healthy = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'Disposable PostgreSQL did not become healthy.' }
    Invoke-Docker @('exec', '--user', 'vscode', '--workdir', '/workspace/rust',
        '--env', "CITADEL_EXECUTION_DATABASE_URL=postgres://citadel_execution:citadel_execution@${postgres}:5432/citadel_execution",
        '--env', "CITADEL_METADATA_DATABASE_URL=postgres://citadel_execution:citadel_execution@${postgres}:5432/citadel_execution",
        '--env', "CITADEL_DENO_PATH=$toolsDirectory/deno", '--env', "CITADEL_SHOUTRRR_PATH=$toolsDirectory/shoutrrr",
        $WorkspaceContainer, 'cargo', 'test', '--locked', '-p', 'citadel-adapters', '--test', 'automation_external_acceptance', '-p', 'citadel-server', '--test', 'automation_http_execution', '--test', 'resources_http', '--test', 'execution_resources_http',
        '--', '--ignored', '--test-threads=1')
}
finally {
    & docker rm --force --volumes $postgres $toolsContainer 2>$null | Out-Null
    if ($toolsDirectory -match '^/tmp/citadel-execution-tools-[a-f0-9]{12}$') {
        & docker exec --user '1000:0' $WorkspaceContainer rm -rf -- $toolsDirectory
    }
}
