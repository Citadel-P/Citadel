param(
    [string]$PostgresImage = 'postgres@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-platform-$suffix"
$postgres = "citadel-rust-platform-postgres-$suffix"
$database = 'citadel_platform'
$rustImage = 'rust@sha256:48b7f435b8fc6378ce43b44f15e3a47fd8919b2825f835c39bd4bd3da50af5b8'

function Invoke-RustTest {
    param([string[]]$CargoArguments)

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/target" `
        --workdir /source `
        --env "CITADEL_PLATFORM_DATABASE_URL=postgres://citadel_platform:citadel_platform@${postgres}:5432/$database" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Rust platform read test failed: cargo $($CargoArguments -join ' ')"
    }
}

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the platform read test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_platform' `
        --env 'POSTGRES_PASSWORD=citadel_platform' `
        --env "POSTGRES_DB=$database" `
        --health-cmd "pg_isready -U citadel_platform -d $database" `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the platform read PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The platform read PostgreSQL fixture did not become healthy.' }

    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--lib')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-docker', '--test', 'docker_transport')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'agent_transport')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'platform_inventory_persistence', '--', '--ignored', '--test-threads=1')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-server', '--lib')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-server', '--test', 'platforms_http', '--', '--ignored', '--test-threads=1')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-server', '--test', 'platform_creation_http', '--', '--ignored', '--test-threads=1')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-server', '--test', 'realtime_subscription', '--', '--include-ignored')
}
finally {
    & docker rm --force $postgres 2>$null | Out-Null
    & docker network rm $network 2>$null | Out-Null
    Remove-CitadelBuildCache -Name $buildCacheVolume
}
