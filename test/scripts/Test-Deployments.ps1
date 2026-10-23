param(
    [string]$PostgresImage = 'postgres@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-deployments-$suffix"
$postgres = "citadel-rust-deployments-postgres-$suffix"
$database = 'citadel_deployments'
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
        --env "CITADEL_WORKLOAD_DATABASE_URL=postgres://citadel_deployments:citadel_deployments@${postgres}:5432/$database" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Rust deployment test failed: cargo $($CargoArguments -join ' ')"
    }
}

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the deployment test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_deployments' `
        --env 'POSTGRES_PASSWORD=citadel_deployments' `
        --env "POSTGRES_DB=$database" `
        --health-cmd "pg_isready -U citadel_deployments -d $database" `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the deployment PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The deployment PostgreSQL fixture did not become healthy.' }

    Invoke-RustTest @('test', '--locked', '-p', 'citadel-deployments')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-primitives', '-p', 'citadel-activities')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-docker', '--test', 'docker_transport')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'agent_mutations')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'deployment_persistence', '--', '--ignored', '--test-threads=1')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-server', '--lib')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-server', '--test', 'deployments_http', '--', '--ignored', '--test-threads=1')
    Invoke-RustTest @('run', '--locked', '-p', 'xtask', '--', 'openapi', '--check')
    Invoke-RustTest @('run', '--locked', '-p', 'xtask', '--', 'docker', '--check')
}
finally {
    & docker rm --force $postgres 2>$null | Out-Null
    & docker network rm $network 2>$null | Out-Null
    Remove-CitadelBuildCache -Name $buildCacheVolume
}
