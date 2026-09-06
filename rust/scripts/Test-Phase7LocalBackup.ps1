param(
    [string]$DevContainer = 'citadel_devcontainer-workspace-1',
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44',
    [switch]$UseRustFs,
    [string]$RustFsImage = 'rustfs/rustfs@sha256:60f4f2f41ce95216f8cac676e69f9d90c0bfec458a3bc7fd7fb9b7c2452ac57a'
)

$ErrorActionPreference = 'Stop'
$inspection = & docker inspect $DevContainer
if ($LASTEXITCODE -ne 0) { throw 'Start the Citadel development container first.' }
$workspace = ($inspection | ConvertFrom-Json)[0]
if (-not $workspace.State.Running) { throw 'The development container is not running.' }
$network = @($workspace.NetworkSettings.Networks.PSObject.Properties.Name)[0]
if (-not $network) { throw 'The development container has no Docker network.' }
$testUid = (& docker exec --user vscode $DevContainer id -u).Trim()
if ($LASTEXITCODE -ne 0 -or $testUid -notmatch '^[1-9][0-9]*$') {
    throw 'Could not resolve the non-root development user.'
}
$postgres = 'citadel-phase7-local-backup-' + [Guid]::NewGuid().ToString('N')
$rustfs = 'citadel-phase7-rustfs-' + [Guid]::NewGuid().ToString('N')
$rustfsStarted = $false
$rustfsEndpoint = ''

try {
    & docker pull restic/restic:0.18.1
    if ($LASTEXITCODE -ne 0) { throw 'Could not obtain the pinned Restic test image.' }
    & docker run --detach --name $postgres --network $network `
        --tmpfs /var/lib/postgresql `
        --env POSTGRES_USER=citadel_test --env POSTGRES_PASSWORD=citadel_test `
        --env POSTGRES_DB=citadel_test $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the disposable backup test database.' }
    $ready = $false
    foreach ($attempt in 1..40) {
        & docker exec $postgres pg_isready -U citadel_test -d citadel_test 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) { throw 'The disposable database did not become ready.' }

    if ($UseRustFs) {
        & docker run --detach --name $rustfs --network $network --publish 9000 `
            --tmpfs /data:rw,mode=1777 `
            --env RUSTFS_ACCESS_KEY=citadel-acceptance-access `
            --env RUSTFS_SECRET_KEY=citadel-acceptance-secret-not-for-production `
            $RustFsImage | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Could not start the disposable RustFS fixture.' }
        $rustfsStarted = $true
        $rustfsInfo = (& docker inspect $rustfs | ConvertFrom-Json)[0]
        $rustfsPort = $rustfsInfo.NetworkSettings.Ports.'9000/tcp'[0].HostPort
        # Restic helper containers use the daemon's default network. The published
        # port lets both the development workspace and those helpers reach RustFS.
        $rustfsEndpoint = "http://host.docker.internal:$rustfsPort"
        $ready = $false
        foreach ($attempt in 1..40) {
            & docker exec $DevContainer curl --silent --fail --max-time 2 `
                --aws-sigv4 aws:amz:us-east-1:s3 `
                --header 'x-amz-content-sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' `
                --user citadel-acceptance-access:citadel-acceptance-secret-not-for-production `
                "$rustfsEndpoint/" 2>$null | Out-Null
            if ($LASTEXITCODE -eq 0) { $ready = $true; break }
            Start-Sleep -Milliseconds 500
        }
        if (-not $ready) {
            & docker logs --tail 30 $rustfs
            & docker exec $DevContainer curl --silent --show-error --max-time 3 "$rustfsEndpoint/"
            throw 'RustFS did not become ready.'
        }
        & docker exec $DevContainer curl --silent --show-error --fail --max-time 10 `
            --aws-sigv4 aws:amz:us-east-1:s3 `
            --header 'x-amz-content-sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' `
            --user citadel-acceptance-access:citadel-acceptance-secret-not-for-production `
            --request PUT "$rustfsEndpoint/citadel-backups"
        if ($LASTEXITCODE -ne 0) { throw 'Could not create the disposable backup bucket.' }
    }

    # Retain the vscode UID for cache ownership; group 0 can read Desktop's
    # real socket even in a workspace created before the proxy fix.
    & docker exec --user "${testUid}:0" --workdir /workspace/rust `
        --env DOCKER_HOST=unix:///var/run/docker-host.sock `
        --env "CITADEL_PHASE7_LOCAL_BACKUP_DATABASE_URL=postgres://citadel_test:citadel_test@${postgres}:5432/citadel_test" `
        --env "CITADEL_PHASE7_RUSTFS_ENDPOINT=$rustfsEndpoint" `
        $DevContainer cargo test --locked -p citadel-adapters --test backup_restic_acceptance $(if ($UseRustFs) { 'rustfs_volume' } else { 'local_volume' }) -- --ignored
    if ($LASTEXITCODE -ne 0) { throw 'The real Local Docker/Restic backup acceptance test failed.' }
}
finally {
    # This UUID-named container owns only the disposable test database.
    & docker rm --force --volumes $postgres 2>$null | Out-Null
    if ($rustfsStarted) { & docker rm --force --volumes $rustfs 2>$null | Out-Null }
}
