param(
    [double]$DurationHours = 24,
    [int]$SampleSeconds = 15,
    [int]$WorkloadSeconds = 60,
    [string]$Image = 'citadel-measurement:local',
    [switch]$SkipBuild,
    [switch]$KeepRunning
)

$ErrorActionPreference = 'Stop'

function Get-Metric([string]$Text, [string]$Name) {
    $line = $Text -split "`n" | Where-Object { $_ -match "^$([regex]::Escape($Name)) " } | Select-Object -First 1
    if (-not $line) { return 0 }
    return [double]::Parse(($line -split ' ')[1], [Globalization.CultureInfo]::InvariantCulture)
}

function Get-Percentile([double[]]$Values, [double]$Percentile) {
    if ($Values.Count -eq 0) { return 0 }
    $sorted = @($Values | Sort-Object)
    $index = [math]::Ceiling($Percentile * $sorted.Count) - 1
    return $sorted[[math]::Max(0, $index)]
}

function Get-SlopeMiBPerHour([System.Collections.Generic.List[object]]$Rows) {
    if ($Rows.Count -lt 2) { return 0 }
    $xs = @($Rows | ForEach-Object { [double]$_.elapsedSeconds / 3600.0 })
    $ys = @($Rows | ForEach-Object { [double]$_.memoryCurrentBytes / 1MB })
    $xMean = ($xs | Measure-Object -Average).Average
    $yMean = ($ys | Measure-Object -Average).Average
    $numerator = 0.0
    $denominator = 0.0
    for ($index = 0; $index -lt $xs.Count; $index++) {
        $xDelta = $xs[$index] - $xMean
        $numerator += $xDelta * ($ys[$index] - $yMean)
        $denominator += $xDelta * $xDelta
    }
    if ($denominator -eq 0) { return 0 }
    return [math]::Round($numerator / $denominator, 6)
}

function Get-CgroupSnapshot {
    $script = @'
if [ -f /sys/fs/cgroup/memory.current ]; then
  echo version=2
  echo current=$(cat /sys/fs/cgroup/memory.current)
  echo peak=$(cat /sys/fs/cgroup/memory.peak)
  echo anon=$(grep '^anon ' /sys/fs/cgroup/memory.stat | cut -d ' ' -f 2)
  echo file=$(grep '^file ' /sys/fs/cgroup/memory.stat | cut -d ' ' -f 2)
  echo pids=$(cat /sys/fs/cgroup/pids.current)
  echo limit=$(cat /sys/fs/cgroup/memory.max)
  echo swapLimit=$(cat /sys/fs/cgroup/memory.swap.max)
else
  echo version=1
  echo current=$(cat /sys/fs/cgroup/memory/memory.usage_in_bytes)
  echo peak=$(cat /sys/fs/cgroup/memory/memory.max_usage_in_bytes)
  echo anon=$(grep '^total_rss ' /sys/fs/cgroup/memory/memory.stat | cut -d ' ' -f 2)
  echo file=$(grep '^total_cache ' /sys/fs/cgroup/memory/memory.stat | cut -d ' ' -f 2)
  echo pids=$(cat /sys/fs/cgroup/pids/pids.current)
  echo limit=$(cat /sys/fs/cgroup/memory/memory.limit_in_bytes)
  echo swapLimit=$(cat /sys/fs/cgroup/memory/memory.memsw.limit_in_bytes)
fi
'@
    $lines = & docker exec citadel-rust-measurement sh -c $script
    if ($LASTEXITCODE -ne 0) { throw 'Could not read container cgroup counters.' }
    $values = @{}
    foreach ($line in $lines) {
        $parts = $line -split '=', 2
        if ($parts.Count -eq 2) { $values[$parts[0]] = $parts[1] }
    }
    foreach ($required in @('version', 'current', 'peak', 'anon', 'file', 'pids', 'limit', 'swapLimit')) {
        if (-not $values.ContainsKey($required) -or [string]::IsNullOrWhiteSpace($values[$required])) {
            throw "Missing cgroup counter '$required'."
        }
    }
    if ($values.limit -eq 'max') { throw 'The measurement memory cgroup is not limited.' }
    if ($values.swapLimit -eq 'max') { throw 'The measurement swap cgroup is not limited.' }
    return $values
}

if ($DurationHours -le 0) { throw 'DurationHours must be greater than zero.' }
if ($SampleSeconds -lt 1) { throw 'SampleSeconds must be at least one.' }
if ($WorkloadSeconds -lt $SampleSeconds) { throw 'WorkloadSeconds must not be shorter than SampleSeconds.' }

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$compose = Join-Path $repoRoot 'deploy/compose.measurement.yml'
$artifactDirectory = Join-Path $repoRoot 'artifacts'
$samplesPath = Join-Path $artifactDirectory 'soak.csv'
$summaryPath = Join-Path $artifactDirectory 'soak-summary.json'
New-Item -ItemType Directory -Force -Path $artifactDirectory | Out-Null
Remove-Item -LiteralPath $samplesPath -ErrorAction SilentlyContinue

if (-not $SkipBuild) {
    & (Join-Path $PSScriptRoot 'Build-MeasurementImage.ps1') -Image $Image
}

$env:CITADEL_MEASUREMENT_IMAGE = $Image
$started = [DateTime]::UtcNow
$deadline = $started.AddHours($DurationHours)
$nextWorkload = $started
$samples = [System.Collections.Generic.List[object]]::new()

try {
    & docker compose --project-name citadel-measurement --file $compose up --detach --wait --no-build
    if ($LASTEXITCODE -ne 0) { throw 'measurement environment failed to start.' }

    while ([DateTime]::UtcNow -lt $deadline) {
        $now = [DateTime]::UtcNow
        if ($now -ge $nextWorkload) {
            & docker exec citadel-rust-measurement /app/citadel-server docker-smoke | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'measurement representative workload failed.' }
            $nextWorkload = $now.AddSeconds($WorkloadSeconds)
        }

        $latency = [System.Diagnostics.Stopwatch]::StartNew()
        $ready = Invoke-WebRequest -UseBasicParsing 'http://127.0.0.1:18000/ready'
        $latency.Stop()
        if ($ready.StatusCode -ne 200) { throw "Readiness returned $($ready.StatusCode)." }

        $metricContent = (Invoke-WebRequest -UseBasicParsing 'http://127.0.0.1:18000/metrics').Content
        $metricText = if ($metricContent -is [byte[]]) {
            [Text.Encoding]::UTF8.GetString($metricContent)
        } else {
            [string]$metricContent
        }
        $dockerStats = (& docker stats citadel-rust-measurement --no-stream --format '{{.CPUPerc}}').TrimEnd('%')
        $cgroup = Get-CgroupSnapshot
        $sample = [pscustomobject]@{
            timestampUtc = $now.ToString('O')
            elapsedSeconds = [math]::Round(($now - $started).TotalSeconds, 3)
            cgroupVersion = [int]$cgroup.version
            memoryCurrentBytes = [int64]$cgroup.current
            memoryPeakBytes = [int64]$cgroup.peak
            memoryAnonBytes = [int64]$cgroup.anon
            memoryFileBytes = [int64]$cgroup.file
            memoryLimitBytes = [int64]$cgroup.limit
            memorySwapLimitBytes = [int64]$cgroup.swapLimit
            cpuPercent = [double]::Parse($dockerStats, [Globalization.CultureInfo]::InvariantCulture)
            healthLatencyMilliseconds = [math]::Round($latency.Elapsed.TotalMilliseconds, 3)
            processCount = [int]$cgroup.pids
            fileDescriptorCount = [int](& docker exec citadel-rust-measurement sh -c 'find /proc/1/fd -maxdepth 1 -type l | wc -l')
            unixSocketCount = [int](& docker exec citadel-rust-measurement sh -c 'tail -n +2 /proc/1/net/unix | wc -l')
            activeTasks = Get-Metric $metricText 'citadel_active_tasks'
            postgresPoolSize = Get-Metric $metricText 'citadel_postgres_pool_size'
            postgresPoolIdle = Get-Metric $metricText 'citadel_postgres_pool_idle'
            eventQueueDepth = Get-Metric $metricText 'citadel_event_queue_depth'
            eventLagMilliseconds = Get-Metric $metricText 'citadel_event_lag_milliseconds'
            eventReconnects = Get-Metric $metricText 'citadel_docker_stream_reconnects_total'
        }
        $samples.Add($sample)
        $sample | Export-Csv -LiteralPath $samplesPath -Append -NoTypeInformation
        Start-Sleep -Seconds $SampleSeconds
    }

    $stopWatch = [System.Diagnostics.Stopwatch]::StartNew()
    & docker stop --time 10 citadel-rust-measurement | Out-Null
    $stopWatch.Stop()
    $inspection = & docker inspect citadel-rust-measurement | ConvertFrom-Json
    $memory = @($samples | ForEach-Object { [double]$_.memoryCurrentBytes })
    $memoryPeak = @($samples | ForEach-Object { [double]$_.memoryPeakBytes })
    $memoryAnon = @($samples | ForEach-Object { [double]$_.memoryAnonBytes })
    $memoryFile = @($samples | ForEach-Object { [double]$_.memoryFileBytes })
    $latencies = @($samples | ForEach-Object { [double]$_.healthLatencyMilliseconds })
    $cpu = @($samples | ForEach-Object { [double]$_.cpuPercent })
    $eventLag = @($samples | ForEach-Object { [double]$_.eventLagMilliseconds })
    $summary = [ordered]@{
        measuredAtUtc = [DateTime]::UtcNow.ToString('O')
        durationHours = $DurationHours
        samples = $samples.Count
        memoryCurrentP95Bytes = Get-Percentile $memory 0.95
        memoryCurrentP99Bytes = Get-Percentile $memory 0.99
        memoryCurrentMaxBytes = ($memory | Measure-Object -Maximum).Maximum
        memoryPeakMaxBytes = ($memoryPeak | Measure-Object -Maximum).Maximum
        memoryAnonMaxBytes = ($memoryAnon | Measure-Object -Maximum).Maximum
        memoryFileMaxBytes = ($memoryFile | Measure-Object -Maximum).Maximum
        memoryRetainedSlopeMiBPerHour = Get-SlopeMiBPerHour $samples
        cpuP95Percent = Get-Percentile $cpu 0.95
        cpuMaxPercent = ($cpu | Measure-Object -Maximum).Maximum
        healthLatencyP95Milliseconds = Get-Percentile $latencies 0.95
        activeTasksMin = ($samples.activeTasks | Measure-Object -Minimum).Minimum
        activeTasksMax = ($samples.activeTasks | Measure-Object -Maximum).Maximum
        processCountMax = ($samples.processCount | Measure-Object -Maximum).Maximum
        fileDescriptorCountMax = ($samples.fileDescriptorCount | Measure-Object -Maximum).Maximum
        unixSocketCountMax = ($samples.unixSocketCount | Measure-Object -Maximum).Maximum
        postgresPoolSizeMax = ($samples.postgresPoolSize | Measure-Object -Maximum).Maximum
        postgresPoolIdleMin = ($samples.postgresPoolIdle | Measure-Object -Minimum).Minimum
        eventQueueDepthMax = ($samples.eventQueueDepth | Measure-Object -Maximum).Maximum
        eventLagP95Milliseconds = Get-Percentile $eventLag 0.95
        eventLagMaxMilliseconds = ($eventLag | Measure-Object -Maximum).Maximum
        eventReconnectsMax = ($samples.eventReconnects | Measure-Object -Maximum).Maximum
        shutdownMilliseconds = [math]::Round($stopWatch.Elapsed.TotalMilliseconds, 3)
        oomKilled = [bool]$inspection[0].State.OOMKilled
        exitCode = [int]$inspection[0].State.ExitCode
        hardLimitBytes = 100MB
    }
    $summary | ConvertTo-Json | Set-Content -LiteralPath $summaryPath
    $summary | Format-List

    if ($summary.oomKilled) { throw 'measurement server was OOM-killed.' }
    if ($samples.Count -eq 0 -or $summary.memoryCurrentMaxBytes -le 0) { throw 'measurement memory counters were not collected.' }
    if (($samples | Where-Object { $_.memoryLimitBytes -ne $summary.hardLimitBytes }).Count -gt 0) { throw 'measurement did not run under the 100 MiB cgroup limit.' }
    if (($samples | Where-Object { $_.memorySwapLimitBytes -ne $_.memoryLimitBytes -and $_.memorySwapLimitBytes -ne 0 }).Count -gt 0) { throw 'measurement swap was not disabled.' }
    if ($summary.memoryCurrentMaxBytes -gt $summary.hardLimitBytes) { throw 'measurement server exceeded the 100 MiB hard limit.' }
    if ($summary.exitCode -ne 0 -and $summary.exitCode -ne 143) { throw "measurement server exited with $($summary.exitCode)." }
}
finally {
    if (-not $KeepRunning) {
        & docker compose --project-name citadel-measurement --file $compose down
    }
    Remove-Item Env:CITADEL_MEASUREMENT_IMAGE -ErrorAction SilentlyContinue
}
