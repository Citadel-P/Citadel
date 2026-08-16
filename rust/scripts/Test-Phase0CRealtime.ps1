param(
    [string]$CoreImage = 'citadel-phase0c:local',
    [string]$AgentImage = 'citadel-agent:phase0c',
    [string]$AgentRepository = 'D:\Projects\Citadel.Agent',
    [int]$DurationSeconds = 45,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
if ($DurationSeconds -lt 10) { throw 'DurationSeconds must be at least 10.' }

function Get-Metric([string]$Text, [string]$Name) {
    $match = [regex]::Match($Text, "(?m)^$([regex]::Escape($Name)) ([0-9]+)\s*$")
    if (-not $match.Success) { throw "Metric '$Name' was not emitted." }
    return [int64]$match.Groups[1].Value
}

function Get-MemoryCurrent([string]$Container) {
    $value = & docker exec $Container sh -c 'if [ -f /sys/fs/cgroup/memory.current ]; then cat /sys/fs/cgroup/memory.current; else cat /sys/fs/cgroup/memory/memory.usage_in_bytes; fi'
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($value)) {
        throw 'Could not read the Phase 0C cgroup memory counter.'
    }
    return [int64]$value
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$agentRoot = (Resolve-Path $AgentRepository).Path
$artifactDirectory = Join-Path $repoRoot 'rust\artifacts\phase0c'
$clientScript = Join-Path $PSScriptRoot 'phase0c-client.mjs'
$runSuffix = [Guid]::NewGuid().ToString('N').Substring(0, 8)
$actorId = [Guid]::NewGuid()
$platformId = [Guid]::NewGuid()
$resourceAccessId = [Guid]::NewGuid()
$networkName = "citadel-rust-phase0c-$runSuffix"
$agentContainer = "citadel-rust-phase0c-agent-$runSuffix"
$postgresContainer = "citadel-rust-phase0c-postgres-$runSuffix"
$serverContainer = "citadel-rust-phase0c-server-$runSuffix"
$fixturePrefix = "citadel-rust-phase0c-fixture-$runSuffix"
$keyFileName = "agent-private-$runSuffix.key"
$keyPath = Join-Path $artifactDirectory $keyFileName
$fixtureSqlPath = Join-Path $artifactDirectory "fixture-$runSuffix.sql"
$clientOutput = Join-Path $artifactDirectory "client-$runSuffix.json"
$clientError = Join-Path $artifactDirectory "client-$runSuffix.err"
$summaryPath = Join-Path $artifactDirectory 'phase0c-workload.json'
$fixtureContainers = [System.Collections.Generic.List[string]]::new()
$memorySamples = [System.Collections.Generic.List[int64]]::new()
New-Item -ItemType Directory -Force -Path $artifactDirectory | Out-Null

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw 'Node.js with the built-in WebSocket API is required for the language-neutral Phase 0C client.'
}
if (-not $SkipBuild) {
    & docker build --file (Join-Path $repoRoot 'rust\Dockerfile.phase0a') --tag $CoreImage $repoRoot
    if ($LASTEXITCODE -ne 0) { throw 'Phase 0C Rust image build failed.' }
    & docker build --file (Join-Path $agentRoot 'Dockerfile') --tag $AgentImage $agentRoot
    if ($LASTEXITCODE -ne 0) { throw 'Active .NET Agent image build failed.' }
}

$privateKey = [byte[]]::new(32)
$tokenBytes = [byte[]]::new(32)
$random = [Security.Cryptography.RandomNumberGenerator]::Create()
try {
    $random.GetBytes($privateKey)
    $random.GetBytes($tokenBytes)
}
finally {
    $random.Dispose()
}
[IO.File]::WriteAllBytes($keyPath, $privateKey)
$realtimeToken = [Convert]::ToBase64String($tokenBytes)
[Array]::Clear($privateKey, 0, $privateKey.Length)
[Array]::Clear($tokenBytes, 0, $tokenBytes.Length)

try {
    & docker network create $networkName | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the Phase 0C Docker network.' }

    $publicKey = & docker run --rm `
        --mount "type=bind,source=$artifactDirectory,target=/phase0c-key,readonly" `
        $CoreImage agent-public-key --private-key-path "/phase0c-key/$keyFileName"
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($publicKey)) {
        throw 'Could not derive the Agent public key with the Rust implementation.'
    }

    & docker run --detach --name $agentContainer --network $networkName `
        --env "HUB_PUBLIC_KEY=$($publicKey.Trim())" `
        --env 'CITADEL_AGENT_TLS_MODE=Disabled' `
        --mount 'type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock' `
        $AgentImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the active .NET Agent.' }

    & docker run --detach --name $postgresContainer --network $networkName `
        --env 'POSTGRES_USER=phase0' `
        --env 'POSTGRES_PASSWORD=phase0' `
        --env 'POSTGRES_DB=phase0' `
        --health-cmd 'pg_isready -U phase0 -d phase0' `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the Phase 0C PostgreSQL fixture.' }
    for ($attempt = 1; $attempt -le 30; $attempt++) {
        if ((& docker inspect $postgresContainer --format '{{.State.Health.Status}}') -eq 'healthy') { break }
        if ($attempt -eq 30) { throw 'The Phase 0C PostgreSQL fixture did not become healthy.' }
        Start-Sleep -Seconds 1
    }

    & docker run --rm --network $networkName `
        --env "DATABASE_URL=postgres://phase0:phase0@${postgresContainer}:5432/phase0" `
        --env 'Transport__Mode=Disabled' `
        $CoreImage migrate | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not apply the Rust database baseline.' }

    $fixtureSql = @"
INSERT INTO Actors (Id, IsEnabled, Type) VALUES ('$actorId', true, 'User');
INSERT INTO Platforms (Id, Name, Address, Status, ConnectorType, CpuCount, ImageCount, MemTotal, NetworkCount, PlatformDescriptor, VolumeCount)
VALUES ('$platformId', 'phase0-local', 'unix:///var/run/docker.sock', 'Healthy', 'Local', 0, 0, 0, 0, '{}', 0);
INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, PermissionLevel, SpecificPermissions)
VALUES ('$resourceAccessId', '$platformId', '$actorId', 0, 1, 0);
UPDATE InstanceSetupStates SET InitializedAt = CURRENT_TIMESTAMP, UpdatedAt = CURRENT_TIMESTAMP WHERE Id = 1;
"@
    [IO.File]::WriteAllText($fixtureSqlPath, $fixtureSql, [Text.UTF8Encoding]::new($false))
    & docker cp $fixtureSqlPath "${postgresContainer}:/tmp/phase0c.sql"
    if ($LASTEXITCODE -ne 0) { throw 'Could not copy the Phase 0C database fixture.' }
    & docker exec $postgresContainer psql --username phase0 --dbname phase0 --file /tmp/phase0c.sql | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the Phase 0C authorization fixture.' }

    for ($index = 0; $index -lt 10; $index++) {
        $name = "$fixturePrefix-running-$index"
        $fixtureContainers.Add($name)
        & docker run --detach --name $name --label 'com.citadel.system=true' --entrypoint sleep $CoreImage 3600 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Could not create running fixture '$name'." }
    }
    for ($index = 0; $index -lt 10; $index++) {
        $name = "$fixturePrefix-stopped-$index"
        $fixtureContainers.Add($name)
        & docker create --name $name --label 'com.citadel.system=true' --entrypoint true $CoreImage | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Could not create stopped fixture '$name'." }
    }

    & docker run --detach --name $serverContainer --network $networkName `
        --publish '127.0.0.1::8000' `
        --env "DATABASE_URL=postgres://phase0:phase0@${postgresContainer}:5432/phase0" `
        --env 'Transport__Mode=Disabled' `
        --env "CITADEL_RUST_AGENT_ADDRESS=http://${agentContainer}:9000" `
        --env "CITADEL_RUST_AGENT_PRIVATE_KEY_PATH=/phase0c-key/$keyFileName" `
        --env 'CITADEL_RUST_AGENT_TIMEOUT_SECONDS=30' `
        --env 'AgentTransport__AllowInsecure=true' `
        --env 'JobConfiguration__MonitoringInterval=1' `
        --env "CITADEL_RUST_REALTIME_ACTOR_ID=$actorId" `
        --env "CITADEL_RUST_REALTIME_PLATFORM_ID=$platformId" `
        --env "CITADEL_RUST_REALTIME_TOKEN=$realtimeToken" `
        --env 'CITADEL_RUST_REALTIME_QUEUE_CAPACITY=4' `
        --env 'CITADEL_RUST_REALTIME_MAX_CONNECTIONS=4' `
        --env 'CITADEL_RUST_REALTIME_AUTH_RECHECK_SECONDS=2' `
        --memory 100m --memory-swap 100m --pids-limit 128 `
        --read-only --tmpfs '/tmp:size=8m,mode=1777' `
        --mount "type=bind,source=$artifactDirectory,target=/phase0c-key,readonly" `
        --mount 'type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock,readonly' `
        $CoreImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the combined Phase 0C server workload.' }

    $publishedAddress = & docker port $serverContainer '8000/tcp'
    $serverPort = ($publishedAddress -split ':')[-1]
    $serverOrigin = "http://127.0.0.1:$serverPort"
    $webSocketUrl = "ws://127.0.0.1:$serverPort/phase0/realtime"
    $metrics = $null
    for ($attempt = 1; $attempt -le 60; $attempt++) {
        try {
            $ready = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/ready"
            $metricsContent = (Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/metrics").Content
            $metrics = if ($metricsContent -is [byte[]]) { [Text.Encoding]::UTF8.GetString($metricsContent) } else { [string]$metricsContent }
            if ($ready.StatusCode -eq 200 `
                -and (Get-Metric $metrics 'citadel_agent_stats_samples_total') -gt 0 `
                -and (Get-Metric $metrics 'citadel_local_stats_samples_total') -gt 0) { break }
        }
        catch {}
        if ($attempt -eq 60) {
            & docker logs $serverContainer
            & docker logs $agentContainer
            throw 'The combined Phase 0C workload did not become ready with Agent and local statistics.'
        }
        Start-Sleep -Seconds 1
    }

    $env:CITADEL_PHASE0C_TOKEN = $realtimeToken
    & node $clientScript --url $webSocketUrl --platform-id $platformId --mode verify |
        Set-Content -LiteralPath (Join-Path $artifactDirectory 'protocol-verification.json')
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 0C protocol verification client failed.' }

    $clientProcess = Start-Process -FilePath node -ArgumentList @(
        $clientScript, '--url', $webSocketUrl, '--platform-id', $platformId,
        '--mode', 'workload', '--duration-seconds', $DurationSeconds
    ) -RedirectStandardOutput $clientOutput -RedirectStandardError $clientError -WindowStyle Hidden -PassThru

    $eventIndex = 0
    while (-not $clientProcess.HasExited) {
        $target = "$fixturePrefix-running-$($eventIndex % 10)"
        & docker pause $target | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Could not pause event fixture '$target'." }
        & docker unpause $target | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Could not unpause event fixture '$target'." }
        $memorySamples.Add((Get-MemoryCurrent $serverContainer))
        $eventIndex += 1
        Start-Sleep -Milliseconds 100
        $clientProcess.Refresh()
    }
    $clientProcess.WaitForExit()
    $clientFailure = Get-Content -Raw -LiteralPath $clientError
    if (-not [string]::IsNullOrWhiteSpace($clientFailure)) {
        throw "The Phase 0C slow/reconnecting client failed: $clientFailure"
    }
    try {
        $clientResult = Get-Content -Raw -LiteralPath $clientOutput | ConvertFrom-Json
    }
    catch {
        throw "The Phase 0C slow/reconnecting client produced invalid output: $($_.Exception.Message)"
    }
    if ($clientResult.reconnects -lt 1 -or $clientResult.slowConnections -lt 1) {
        throw 'The Phase 0C client did not exercise both reconnecting and slow-client paths.'
    }

    $metricsContent = (Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/metrics").Content
    $metrics = if ($metricsContent -is [byte[]]) { [Text.Encoding]::UTF8.GetString($metricsContent) } else { [string]$metricsContent }
    $activeTasks = Get-Metric $metrics 'citadel_active_tasks'
    $activeConnections = Get-Metric $metrics 'citadel_realtime_active_connections'
    $snapshots = Get-Metric $metrics 'citadel_realtime_snapshot_resyncs_total'
    $published = Get-Metric $metrics 'citadel_realtime_events_published_total'
    if ($activeTasks -ne 5) { throw "Expected five owned background tasks, found $activeTasks." }
    if ($activeConnections -ne 0) { throw "Realtime client retention detected: $activeConnections connection(s) remain." }
    if ($snapshots -lt 3) { throw "Expected reconnect snapshots, found $snapshots." }
    if ($published -eq 0) { throw 'No runtime events were published to realtime.' }
    if ((& docker inspect $serverContainer --format '{{.State.OOMKilled}}') -ne 'false') {
        throw 'The combined Phase 0C workload exceeded its 100 MiB memory limit.'
    }

    & docker stop --time 10 $serverContainer | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The combined Phase 0C server did not stop cleanly.' }
    $serverExitCode = [int](& docker inspect $serverContainer --format '{{.State.ExitCode}}')
    if ($serverExitCode -ne 0) {
        & docker logs $serverContainer
        throw "The combined Phase 0C server exited with code $serverExitCode."
    }
    $summary = [ordered]@{
        measuredAtUtc = [DateTime]::UtcNow.ToString('O')
        durationSeconds = $DurationSeconds
        deterministicContainers = $fixtureContainers.Count
        memorySamples = $memorySamples.Count
        memoryCurrentMaxBytes = ($memorySamples | Measure-Object -Maximum).Maximum
        hardLimitBytes = 100MB
        activeTasks = $activeTasks
        activeConnectionsAfterWorkload = $activeConnections
        agentStatsSamples = Get-Metric $metrics 'citadel_agent_stats_samples_total'
        localStatsSamples = Get-Metric $metrics 'citadel_local_stats_samples_total'
        realtimeEventsPublished = $published
        realtimeMessagesSent = Get-Metric $metrics 'citadel_realtime_messages_sent_total'
        realtimeOverflows = Get-Metric $metrics 'citadel_realtime_overflows_total'
        realtimeSnapshots = $snapshots
        realtimeSendTimeouts = Get-Metric $metrics 'citadel_realtime_send_timeouts_total'
        oomKilled = $false
        exitCode = $serverExitCode
    }
    $summary | ConvertTo-Json | Set-Content -LiteralPath $summaryPath
    $summary | Format-List
}
finally {
    Remove-Item Env:CITADEL_PHASE0C_TOKEN -ErrorAction SilentlyContinue
    foreach ($container in $fixtureContainers) {
        if (@(& docker ps --all --format '{{.Names}}') -contains $container) {
            & docker rm --force $container | Out-Null
        }
    }
    foreach ($container in @($serverContainer, $postgresContainer, $agentContainer)) {
        if (@(& docker ps --all --format '{{.Names}}') -contains $container) {
            & docker rm --force $container | Out-Null
        }
    }
    if (@(& docker network ls --format '{{.Name}}') -contains $networkName) {
        & docker network rm $networkName | Out-Null
    }
    foreach ($path in @($keyPath, $fixtureSqlPath)) {
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    }
}
