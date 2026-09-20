param(
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-phase6a-$suffix"
$postgres = "citadel-rust-phase6a-postgres-$suffix"
$database = 'citadel_phase6a'
$rustImage = 'rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97'

function Invoke-RustTest {
    param([string[]]$CargoArguments)

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE6_DATABASE_URL=postgres://citadel_phase6a:citadel_phase6a@${postgres}:5432/$database" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Rust Phase 6A test failed: cargo $($CargoArguments -join ' ')"
    }
}

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the Phase 6A test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_phase6a' `
        --env 'POSTGRES_PASSWORD=citadel_phase6a' `
        --env "POSTGRES_DB=$database" `
        --health-cmd "pg_isready -U citadel_phase6a -d $database" `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the Phase 6A PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The Phase 6A PostgreSQL fixture did not become healthy.' }

    Invoke-RustTest @('test', '--locked', '-p', 'citadel-deployments')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-primitives', '-p', 'citadel-activities')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'docker_transport')
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
