param(
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-phase7a-$suffix"
$postgres = "citadel-rust-phase7a-postgres-$suffix"
$database = 'citadel_phase7a'
$rustImage = 'rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97'

function Invoke-Rust {
    param([string[]]$CargoArguments)

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume 'citadel-rust-target:/source/rust/target' `
        --workdir /source/rust `
        --env "CITADEL_PHASE5_DATABASE_URL=postgres://citadel_phase7a:citadel_phase7a@${postgres}:5432/$database" `
        --env "CITADEL_PHASE7_DATABASE_URL=postgres://citadel_phase7a:citadel_phase7a@${postgres}:5432/$database" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Rust Phase 7A test failed: cargo $($CargoArguments -join ' ')"
    }
}

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the Phase 7A test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_phase7a' `
        --env 'POSTGRES_PASSWORD=citadel_phase7a' `
        --env "POSTGRES_DB=$database" `
        --health-cmd "pg_isready -U citadel_phase7a -d $database" `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the Phase 7A PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The Phase 7A PostgreSQL fixture did not become healthy.' }

    Invoke-Rust @('fmt', '--all', '--', '--check')
    Invoke-Rust @('clippy', '--locked', '-p', 'citadel-execution', '-p', 'citadel-git', '-p', 'citadel-automation', '-p', 'citadel-adapters', '-p', 'citadel-server', '--lib', '--', '-D', 'warnings')
    Invoke-Rust @('clippy', '--locked', '-p', 'citadel-execution', '--test', 'process_runner', '-p', 'citadel-server', '--test', 'resources_http', '--', '-D', 'warnings')
    Invoke-Rust @('test', '--locked', '-p', 'citadel-execution')
    Invoke-Rust @('test', '--locked', '-p', 'citadel-git')
    Invoke-Rust @('test', '--locked', '-p', 'citadel-automation')
    Invoke-Rust @('test', '--locked', '-p', 'citadel-adapters', '--test', 'git_repository_execution', '--test', 'automation_execution', '--', '--ignored', '--test-threads=1')
    Invoke-Rust @('test', '--locked', '-p', 'citadel-server', '--test', 'resources_http', '--', '--ignored', '--test-threads=1')
    Invoke-Rust @('run', '--locked', '-p', 'xtask', '--', 'openapi', '--check')
}
finally {
    & docker rm --force $postgres 2>$null | Out-Null
    & docker network rm $network 2>$null | Out-Null
}
