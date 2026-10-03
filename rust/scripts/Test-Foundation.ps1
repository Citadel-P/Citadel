param(
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-foundation-$suffix"
$postgres = "citadel-rust-foundation-postgres-$suffix"
$server = "citadel-rust-foundation-server-$suffix"
$databaseUrl = "postgres://citadel_foundation:citadel_foundation@${postgres}:5432/citadel_foundation"
$rustImage = 'rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97'

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the foundation test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_foundation' `
        --env 'POSTGRES_PASSWORD=citadel_foundation' `
        --env 'POSTGRES_DB=citadel_foundation' `
        --health-cmd 'pg_isready -U citadel_foundation -d citadel_foundation' `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the foundation PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        $health = & docker inspect $postgres --format '{{.State.Health.Status}}'
        if ($health -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The foundation PostgreSQL fixture did not become healthy.' }

    & docker exec $postgres createdb --username citadel_foundation --owner citadel_foundation citadel_foundation_server
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the foundation server database.' }

    & docker run --rm `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo build --locked -p citadel-server
    if ($LASTEXITCODE -ne 0) { throw 'The foundation server did not build.' }

    & docker run --detach --name $server --network $network `
        --publish '127.0.0.1::8000' `
        --volume "${repoRoot}:/source:ro" `
        --volume "${buildCacheVolume}:/source/rust/target:ro" `
        --volume '/var/run/docker.sock:/var/run/docker.sock:ro' `
        --workdir /source/rust `
        --env "DATABASE_URL=postgres://citadel_foundation:citadel_foundation@${postgres}:5432/citadel_foundation_server" `
        --env 'Transport__Mode=Disabled' `
        --env 'Jwt__Key=foundation-only-jwt-key-at-least-32-bytes' `
        --env 'Secrets__EncryptionKey=AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=' `
        --env 'EnableSwagger=true' `
        --env 'JobConfiguration__MonitoringInterval=1' `
        $rustImage `
        /source/rust/target/debug/citadel-server serve | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The foundation server did not start.' }
    $publishedAddress = & docker port $server '8000/tcp'
    $serverPort = ($publishedAddress -split ':')[-1]
    $serverOrigin = "http://127.0.0.1:$serverPort"
    $serverReady = $false
    foreach ($attempt in 1..60) {
        try {
            $health = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/health"
            $ready = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/ready"
            $openApi = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$serverOrigin/openapi/v1.json"
            if ($health.StatusCode -eq 200 -and $ready.StatusCode -eq 200 -and $openApi.Content -match 'getHealth') {
                $serverReady = $true
                break
            }
        }
        catch {}
        Start-Sleep -Milliseconds 500
    }
    if (-not $serverReady) {
        & docker logs $server
        throw 'The migrated foundation server did not become ready.'
    }
    & docker stop --time 15 $server | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The foundation server did not stop gracefully.' }
    $serverExitCode = [int](& docker inspect $server --format '{{.State.ExitCode}}')
    if ($serverExitCode -ne 0) {
        & docker logs $server
        throw "The foundation server exited with code $serverExitCode during graceful shutdown."
    }
    & docker rm $server | Out-Null

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_TEST_DATABASE_URL=$databaseUrl" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-database --test migrations -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The foundation database integration test failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_AUTHORIZED_READ_DATABASE_URL=$databaseUrl" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-adapters --test authorized_read -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The checked Actor-scoped PostgreSQL query test failed.' }

    & docker run --rm `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        $rustImage `
        sh -c 'cargo run --locked -p xtask -- database verify && cargo run --locked -p xtask -- docker --check && cargo run --locked -p xtask -- openapi --check'
    if ($LASTEXITCODE -ne 0) { throw 'A generated foundation artifact is stale.' }
}
finally {
    $containers = @(& docker ps --all --format '{{.Names}}')
    if ($containers -contains $server) {
        & docker rm --force $server | Out-Null
    }
    if ($containers -contains $postgres) {
        & docker rm --force $postgres | Out-Null
    }
    if (@(& docker network ls --format '{{.Name}}') -contains $network) {
        & docker network rm $network | Out-Null
    }
    Remove-CitadelBuildCache -Name $buildCacheVolume
}
