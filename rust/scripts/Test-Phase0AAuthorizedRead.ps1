$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$compose = Join-Path $repoRoot 'rust\compose.measurement.yml'
$mount = "type=bind,source=$repoRoot,target=/repo"

try {
    & docker compose --project-name citadel-phase0a --file $compose up --detach --wait phase0-postgres
    if ($LASTEXITCODE -ne 0) { throw 'Phase 0A PostgreSQL did not start.' }

    & docker run --rm `
        --network citadel-phase0a `
        --env 'CITADEL_PHASE0_DATABASE_URL=postgres://citadel_phase0:citadel_phase0@phase0-postgres:5432/citadel_phase0' `
        --mount $mount `
        --volume 'citadel-rust-cargo-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-cargo-git:/usr/local/cargo/git' `
        --volume "${buildCacheVolume}:/repo/rust/target" `
        --workdir /repo/rust `
        rust:1.97.1-bookworm `
        cargo test --locked -p citadel-adapters --test authorized_read -- --ignored --exact actor_authorized_platform_read_preserves_direct_and_team_scope
    if ($LASTEXITCODE -ne 0) { throw 'Actor-authorized PostgreSQL fixture failed.' }
}
finally {
    & docker compose --project-name citadel-phase0a --file $compose down --volumes
    Remove-CitadelBuildCache -Name $buildCacheVolume
}
