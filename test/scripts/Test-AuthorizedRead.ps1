$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$compose = Join-Path $repoRoot 'deploy/compose.measurement.yml'
$mount = "type=bind,source=$repoRoot,target=/repo"

try {
    & docker compose --project-name citadel-measurement --file $compose up --detach --wait measurement-postgres
    if ($LASTEXITCODE -ne 0) { throw 'measurement PostgreSQL did not start.' }

    & docker run --rm `
        --network citadel-measurement `
        --env 'CITADEL_AUTHORIZED_READ_DATABASE_URL=postgres://citadel_measurement:citadel_measurement@measurement-postgres:5432/citadel_measurement' `
        --mount $mount `
        --volume 'citadel-rust-cargo-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-cargo-git:/usr/local/cargo/git' `
        --volume "${buildCacheVolume}:/repo/target" `
        --workdir /repo `
        rust:1.97.1-bookworm `
        cargo test --locked -p citadel-adapters --test authorized_read -- --ignored --exact actor_authorized_platform_read_preserves_direct_and_team_scope
    if ($LASTEXITCODE -ne 0) { throw 'Actor-authorized PostgreSQL fixture failed.' }
}
finally {
    & docker compose --project-name citadel-measurement --file $compose down --volumes
    Remove-CitadelBuildCache -Name $buildCacheVolume
}
