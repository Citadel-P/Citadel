param(
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44',
    [switch]$RunSwarmLifecycle
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-workload-stack-$suffix"
$postgres = "citadel-rust-workload-stack-postgres-$suffix"
$database = 'citadel_workload_stack'
$rustImage = 'rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97'
$databaseUrl = "postgres://citadel_workload_stack:citadel_workload_stack@${postgres}:5432/$database"

function Invoke-RustTest {
    param([string[]]$CargoArguments)

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_WORKLOAD_DATABASE_URL=$databaseUrl" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Rust Stack test failed: cargo $($CargoArguments -join ' ')"
    }
}

function Invoke-RuntimeLifecycleTest {
    & docker run --rm --network $network `
        --add-host 'host.docker.internal:host-gateway' `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --volume '/var/run/docker.sock:/var/run/docker.sock' `
        --workdir /source/rust `
        --env "CITADEL_WORKLOAD_DATABASE_URL=$databaseUrl" `
        --env "CITADEL_RUNTIME_IMAGE=$PostgresImage" `
        --env "CITADEL_RUN_SWARM_LIFECYCLE=$($RunSwarmLifecycle.IsPresent.ToString().ToLowerInvariant())" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        sh ./scripts/test-stack-runtime.sh
    if ($LASTEXITCODE -ne 0) {
        throw 'Rust workload real-Docker Stack lifecycle test failed.'
    }
}

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the Stack test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_workload_stack' `
        --env 'POSTGRES_PASSWORD=citadel_workload_stack' `
        --env "POSTGRES_DB=$database" `
        --health-cmd "pg_isready -U citadel_workload_stack -d $database" `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the Stack PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The Stack PostgreSQL fixture did not become healthy.' }

    Invoke-RustTest @('test', '--locked', '-p', 'citadel-stacks')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'agent_mutations')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-adapters', '--test', 'stack_persistence', '--', '--ignored', '--test-threads=1')
    Invoke-RustTest @('test', '--locked', '-p', 'citadel-server', '--test', 'stacks_http', '--', '--ignored', '--test-threads=1')
    Invoke-RuntimeLifecycleTest
    Invoke-RustTest @('run', '--locked', '-p', 'xtask', '--', 'openapi', '--check')
    Invoke-RustTest @('run', '--locked', '-p', 'xtask', '--', 'docker', '--check')
}
finally {
    & docker rm --force $postgres 2>$null | Out-Null
    & docker network rm $network 2>$null | Out-Null
    Remove-CitadelBuildCache -Name $buildCacheVolume
}
