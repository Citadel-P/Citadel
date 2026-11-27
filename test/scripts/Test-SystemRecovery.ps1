param(
    [string]$DevContainer = 'citadel_devcontainer-workspace-1',
    # The Rust test binary built in the workspace requires a GNU client runtime.
    [string]$ClientImage = 'postgres@sha256:4ea94de574c17f0372722806c949757050fbb1cd7648410fe45d6bba275cc00e',
    [string]$PostgresImage = 'postgres@sha256:cac45fb483caee3668946088a1c0532db89d5285c28dbfeac889b4068296e06f'
)

$ErrorActionPreference = 'Stop'
$inspection = & docker inspect $DevContainer
if ($LASTEXITCODE -ne 0) { throw 'Start the development container first.' }
$workspace = ($inspection | ConvertFrom-Json)[0]
$network = @($workspace.NetworkSettings.Networks.PSObject.Properties.Name)[0]
$target = @($workspace.Mounts | Where-Object Destination -eq '/home/vscode/.cache/citadel-target')[0]
if (-not $workspace.State.Running -or -not $network -or $target.Type -ne 'volume') {
    throw 'The running development container must use its standard target cache volume.'
}
& docker exec --user vscode --workdir /workspace $DevContainer cargo test --locked -p citadel-adapters --test citadel_system_recovery --no-run
if ($LASTEXITCODE -ne 0) { throw 'Recovery test build failed.' }
$executables = & docker exec --user vscode $DevContainer find /home/vscode/.cache/citadel-target/debug/deps -maxdepth 1 -name 'citadel_system_recovery-*' -type f -executable
if ($LASTEXITCODE -ne 0 -or @($executables).Count -ne 1) { throw 'Expected one compiled recovery test executable.' }
$testBinary = '/target/debug/deps/' + [IO.Path]::GetFileName($executables.Trim())
$database = 'citadel-system-recovery-' + [Guid]::NewGuid().ToString('N')
try {
    & docker run --detach --name $database --network $network --tmpfs /var/lib/postgresql `
        --env POSTGRES_USER=citadel --env POSTGRES_PASSWORD=citadel --env POSTGRES_DB=source $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Disposable recovery database failed to start.' }
    $ready = $false
    foreach ($attempt in 1..40) {
        & docker exec $database pg_isready -h 127.0.0.1 -U citadel -d source *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) { throw 'Recovery database did not become ready.' }
    & docker exec $database createdb -U citadel target
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the disposable restore target.' }
    # Run as the image's unprivileged user with matching pg_dump/pg_restore.
    # Only compiled artifacts are mounted; no development or user database is accessible.
    & docker run --rm --user postgres --network $network --workdir /tmp `
        --volume "$($target.Name):/target:ro" --entrypoint $testBinary `
        --env "CITADEL_RECOVERY_SOURCE_DATABASE_URL=postgres://citadel:citadel@${database}:5432/source" `
        --env "CITADEL_RECOVERY_TARGET_DATABASE_URL=postgres://citadel:citadel@${database}:5432/target" `
        $ClientImage --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'Citadel system backup/restore acceptance failed.' }
} finally {
    & docker rm --force --volumes $database | Out-Null
}
