param(
    [string]$CoreImage = 'citadel-phase0b:local',
    [string]$AgentImage = 'citadel-agent:phase0b',
    [string]$AgentRepository = 'D:\Projects\Citadel.Agent',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$agentRoot = (Resolve-Path $AgentRepository).Path
$artifactDirectory = Join-Path $repoRoot 'rust\artifacts\phase0b'
$runSuffix = [Guid]::NewGuid().ToString('N').Substring(0, 8)
$keyFileName = "agent-private-$runSuffix.key"
$keyPath = Join-Path $artifactDirectory $keyFileName
$networkName = "citadel-rust-phase0b-$runSuffix"
$agentContainer = "citadel-rust-phase0b-agent-$runSuffix"
$postgresContainer = "citadel-rust-phase0b-postgres-$runSuffix"
$serverContainer = "citadel-rust-phase0b-server-$runSuffix"
New-Item -ItemType Directory -Force -Path $artifactDirectory | Out-Null

if (-not $SkipBuild) {
    & docker build --file (Join-Path $repoRoot 'rust\Dockerfile') --tag $CoreImage $repoRoot
    if ($LASTEXITCODE -ne 0) { throw 'Phase 0B Rust image build failed.' }
    & docker build --file (Join-Path $agentRoot 'Dockerfile') --tag $AgentImage $agentRoot
    if ($LASTEXITCODE -ne 0) { throw 'Active .NET Agent image build failed.' }
}

$privateKey = [byte[]]::new(32)
$random = [Security.Cryptography.RandomNumberGenerator]::Create()
try {
    $random.GetBytes($privateKey)
}
finally {
    $random.Dispose()
}
[IO.File]::WriteAllBytes($keyPath, $privateKey)
[Array]::Clear($privateKey, 0, $privateKey.Length)

try {
    $networkExists = @(& docker network ls --format '{{.Name}}') -contains $networkName
    if (-not $networkExists) {
        & docker network create $networkName | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Could not create the Phase 0B Docker network.' }
    }

    $publicKey = & docker run --rm `
        --mount "type=bind,source=$artifactDirectory,target=/phase0b-key,readonly" `
        $CoreImage agent-public-key --private-key-path "/phase0b-key/$keyFileName"
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($publicKey)) {
        throw 'Could not derive the Agent public key with the Rust implementation.'
    }

    & docker run --detach --name $agentContainer --network $networkName `
        --env "HUB_PUBLIC_KEY=$($publicKey.Trim())" `
        --env 'CITADEL_AGENT_TLS_MODE=Disabled' `
        --mount 'type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock' `
        $AgentImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the active .NET Agent.' }

    $result = $null
    for ($attempt = 1; $attempt -le 30; $attempt++) {
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $result = & docker run --rm --network $networkName `
            --env 'DATABASE_URL=postgres://phase0:phase0@unused:5432/phase0' `
            --env 'Transport__Mode=Disabled' `
            --env "CITADEL_RUST_AGENT_ADDRESS=http://${agentContainer}:9000" `
            --env "CITADEL_RUST_AGENT_PRIVATE_KEY_PATH=/phase0b-key/$keyFileName" `
            --env 'CITADEL_RUST_AGENT_TIMEOUT_SECONDS=30' `
            --env 'AgentTransport__AllowInsecure=true' `
            --env 'JobConfiguration__MonitoringInterval=1' `
            --mount "type=bind,source=$artifactDirectory,target=/phase0b-key,readonly" `
            --mount 'type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock,readonly' `
            $CoreImage phase0-agent-smoke 2>&1
        $smokeExitCode = $LASTEXITCODE
        $ErrorActionPreference = $previousErrorActionPreference
        if ($smokeExitCode -eq 0) { break }
        $resultText = $result | Out-String
        if ($attempt -eq 30 -or $resultText -notmatch 'kind: Unavailable') {
            & docker logs $agentContainer
            throw "The active .NET Agent did not pass Phase 0B interoperability: $resultText"
        }
        Start-Sleep -Milliseconds 500
    }

    $result | Tee-Object -FilePath (Join-Path $artifactDirectory 'agent-interop.json')

    & docker run --detach --name $postgresContainer --network $networkName `
        --env 'POSTGRES_USER=phase0' `
        --env 'POSTGRES_PASSWORD=phase0' `
        --env 'POSTGRES_DB=phase0' `
        --health-cmd 'pg_isready -U phase0 -d phase0' `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the Phase 0B PostgreSQL fixture.' }
    for ($attempt = 1; $attempt -le 30; $attempt++) {
        $postgresHealth = & docker inspect $postgresContainer --format '{{.State.Health.Status}}'
        if ($postgresHealth -eq 'healthy') { break }
        if ($attempt -eq 30) { throw 'The Phase 0B PostgreSQL fixture did not become healthy.' }
        Start-Sleep -Seconds 1
    }

    & docker run --detach --name $serverContainer --network $networkName `
        --publish '127.0.0.1::8000' `
        --env "DATABASE_URL=postgres://phase0:phase0@${postgresContainer}:5432/phase0" `
        --env 'Transport__Mode=Disabled' `
        --env "CITADEL_RUST_AGENT_ADDRESS=http://${agentContainer}:9000" `
        --env "CITADEL_RUST_AGENT_PRIVATE_KEY_PATH=/phase0b-key/$keyFileName" `
        --env 'CITADEL_RUST_AGENT_TIMEOUT_SECONDS=30' `
        --env 'AgentTransport__AllowInsecure=true' `
        --env 'JobConfiguration__MonitoringInterval=1' `
        --memory 100m --memory-swap 100m --pids-limit 128 `
        --read-only --tmpfs '/tmp:size=8m,mode=1777' `
        --mount "type=bind,source=$artifactDirectory,target=/phase0b-key,readonly" `
        --mount 'type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock,readonly' `
        $CoreImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the combined Phase 0B server workload.' }

    $publishedAddress = & docker port $serverContainer '8000/tcp'
    $serverPort = ($publishedAddress -split ':')[-1]
    $serverOrigin = "http://127.0.0.1:$serverPort"
    $metrics = $null
    $lastReadyStatus = $null
    $lastProbeError = $null
    for ($attempt = 1; $attempt -le 60; $attempt++) {
        try {
            $ready = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/ready"
            $lastReadyStatus = $ready.StatusCode
            $metricsContent = (Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/metrics").Content
            $metrics = if ($metricsContent -is [byte[]]) {
                [Text.Encoding]::UTF8.GetString($metricsContent)
            }
            else {
                [string]$metricsContent
            }
            if ($ready.StatusCode -eq 200) {
                $samples = [regex]::Match($metrics, '(?m)^citadel_agent_stats_samples_total ([0-9]+)\s*$')
                if ($samples.Success -and [int64]$samples.Groups[1].Value -gt 0) { break }
            }
        }
        catch {
            $lastProbeError = $_.Exception.Message
        }
        if ($attempt -eq 60) {
            & docker logs $serverContainer
            & docker logs $agentContainer
            throw "The combined Phase 0B workload did not become ready with an Agent sample. Last readiness status: $lastReadyStatus. Last probe error: $lastProbeError. Metrics: $metrics"
        }
        Start-Sleep -Seconds 1
    }

    $activeTasks = [regex]::Match($metrics, '(?m)^citadel_active_tasks ([0-9]+)\s*$')
    if (-not $activeTasks.Success -or [int64]$activeTasks.Groups[1].Value -ne 4) {
        throw 'The combined Phase 0B workload did not own exactly four background tasks.'
    }
    if ((& docker inspect $serverContainer --format '{{.State.OOMKilled}}') -ne 'false') {
        throw 'The combined Phase 0B workload exceeded its 100 MiB memory limit.'
    }

    & docker stop --time 10 $serverContainer | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The combined Phase 0B server did not stop cleanly.' }
    $serverExitCode = [int](& docker inspect $serverContainer --format '{{.State.ExitCode}}')
    if ($serverExitCode -ne 0) {
        & docker logs $serverContainer
        throw "The combined Phase 0B server exited with code $serverExitCode."
    }
    [ordered]@{
        activeTasks = [int64]$activeTasks.Groups[1].Value
        agentStatsSamples = [int64]$samples.Groups[1].Value
        oomKilled = $false
        exitCode = $serverExitCode
    } | ConvertTo-Json | Tee-Object -FilePath (Join-Path $artifactDirectory 'agent-workload.json')
}
finally {
    $serverExists = @(& docker ps --all --format '{{.Names}}') -contains $serverContainer
    if ($serverExists) {
        & docker rm --force $serverContainer | Out-Null
    }
    $postgresExists = @(& docker ps --all --format '{{.Names}}') -contains $postgresContainer
    if ($postgresExists) {
        & docker rm --force $postgresContainer | Out-Null
    }
    $containerExists = @(& docker ps --all --format '{{.Names}}') -contains $agentContainer
    if ($containerExists) {
        & docker rm --force $agentContainer | Out-Null
    }
    $networkExists = @(& docker network ls --format '{{.Name}}') -contains $networkName
    if ($networkExists) {
        & docker network rm $networkName | Out-Null
    }
    if (Test-Path -LiteralPath $keyPath) {
        Remove-Item -LiteralPath $keyPath -Force
    }
}
