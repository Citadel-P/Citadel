param(
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-deployment-apply-$suffix"
$postgres = "citadel-rust-deployment-apply-postgres-$suffix"
$database = 'citadel_deployment_apply'
$rustImage = 'rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97'

function Invoke-RustTest {
    param([string[]]$CargoArguments)

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/target" `
        --volume '/var/run/docker.sock:/var/run/docker.sock' `
        --workdir /source `
        --env "CITADEL_WORKLOAD_DATABASE_URL=postgres://citadel_deployment_apply:citadel_deployment_apply@${postgres}:5432/$database" `
        --env 'SQLX_OFFLINE=true' `
        --env "CITADEL_RUNTIME_IMAGE=$PostgresImage" `
        $rustImage `
        cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Rust deployment apply test failed: cargo $($CargoArguments -join ' ')"
    }
}

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the deployment apply test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_deployment_apply' `
        --env 'POSTGRES_PASSWORD=citadel_deployment_apply' `
        --env "POSTGRES_DB=$database" `
        --health-cmd "pg_isready -U citadel_deployment_apply -d $database" `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the deployment apply PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The deployment apply PostgreSQL fixture did not become healthy.' }

    Invoke-RustTest @('test', '--locked', '-p', 'citadel-deployments')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'docker_transport')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'agent_mutations')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'deployment_apply_persistence', '--', '--ignored', '--test-threads=1')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'deployment_runtime_local', '--', '--ignored', '--test-threads=1')
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
