param(
    [switch]$SkipBuild,
    [ValidatePattern('^citadel-rust-e2e-[a-z0-9-]+$')][string]$BuildCacheVolume,
    [string[]]$TestFiles = @('tests/compatibility/oidc.spec.ts'),
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44',
    [string]$KeycloakImage = 'quay.io/keycloak/keycloak@sha256:0f198be292568439d700cdbfb893e69a6009bb43a94a06a945b1d3d506c76b13'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
if ($SkipBuild -and -not $BuildCacheVolume) {
    throw '-SkipBuild requires an explicit -BuildCacheVolume from a previous build.'
}
$removeBuildCache = -not $BuildCacheVolume
if ($removeBuildCache) { $BuildCacheVolume = New-CitadelBuildCacheName }
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$frontendRoot = Join-Path $repoRoot 'src\Citadel.FrontEnd'
$e2eRoot = Join-Path $repoRoot 'test\Citadel.Tests.E2E'
$realm = Join-Path $e2eRoot 'fixtures\keycloak\citadel-e2e-realm.json'
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-identity-frontend-$suffix"
$postgres = "citadel-rust-identity-frontend-postgres-$suffix"
$keycloak = "citadel-rust-identity-frontend-keycloak-$suffix"
$server = "citadel-rust-identity-frontend-server-$suffix"
$rustImage = 'rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97'
$serverOrigin = 'http://127.0.0.1:18000'
$keycloakOrigin = 'http://127.0.0.1:18080'

function Wait-ForHttp {
    param(
        [string]$Uri,
        [string]$Description
    )

    foreach ($attempt in 1..120) {
        try {
            $response = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri $Uri
            if ($response.StatusCode -eq 200) {
                return
            }
        }
        catch {
            # The disposable service can refuse connections while starting.
        }
        Start-Sleep -Milliseconds 500
    }
    throw "$Description did not become ready at $Uri."
}

try {
    if (-not $SkipBuild) {
        & npm --prefix $frontendRoot run build:image
        if ($LASTEXITCODE -ne 0) { throw 'The Citadel frontend production build failed.' }

        & docker run --rm `
            --volume "${repoRoot}:/source" `
            --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
            --volume 'citadel-rust-git:/usr/local/cargo/git' `
            --volume 'citadel-rustup:/usr/local/rustup' `
            --volume "${BuildCacheVolume}:/source/rust/target" `
            --workdir /source/rust `
            $rustImage `
            cargo build --locked -p citadel-server
        if ($LASTEXITCODE -ne 0) { throw 'The Rust Citadel server build failed.' }
    }

    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the identity frontend test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel' `
        --env 'POSTGRES_PASSWORD=citadel-e2e' `
        --env 'POSTGRES_DB=citadel' `
        --health-cmd 'pg_isready -U citadel -d citadel' `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the identity frontend PostgreSQL fixture.' }

    $postgresHealthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $postgresHealthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $postgresHealthy) { throw 'The identity frontend PostgreSQL fixture did not become healthy.' }

    & docker run --detach --name $keycloak --network $network --network-alias keycloak `
        --publish '127.0.0.1:18080:18080' `
        --env 'KC_BOOTSTRAP_ADMIN_USERNAME=citadel-e2e-admin' `
        --env 'KC_BOOTSTRAP_ADMIN_PASSWORD=citadel-e2e-admin-password' `
        --env 'KC_HEALTH_ENABLED=true' `
        --volume "${realm}:/opt/keycloak/data/import/citadel-e2e-realm.json:ro" `
        $KeycloakImage `
        start-dev --import-realm --http-port=18080 --hostname=http://keycloak:18080 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the identity frontend Keycloak fixture.' }
    Wait-ForHttp "$keycloakOrigin/realms/citadel-e2e/.well-known/openid-configuration" 'Keycloak'

    & docker run --detach --name $server --network $network `
        --publish '127.0.0.1:18000:8000' `
        --volume "${repoRoot}:/source:ro" `
        --volume "${BuildCacheVolume}:/source/rust/target:ro" `
        --volume '/var/run/docker.sock:/var/run/docker.sock:ro' `
        --env "DATABASE_URL=postgres://citadel:citadel-e2e@${postgres}:5432/citadel" `
        --env 'Transport__Mode=Disabled' `
        --env "Transport__PublicUrl=$serverOrigin" `
        --env 'Jwt__Key=citadel-e2e-only-signing-key-00000000000000000000000000000000' `
        --env 'Secrets__EncryptionKey=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=' `
        --env 'CITADEL_RUST_STATIC_ROOT=/source/src/Citadel.FrontEnd/dist' `
        $rustImage `
        /source/rust/target/debug/citadel-server serve | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the identity frontend Rust server.' }
    Wait-ForHttp "$serverOrigin/health" 'Rust Citadel'

    Push-Location $e2eRoot
    try {
        $env:CITADEL_E2E_BASE_URL = $serverOrigin
        $env:CITADEL_E2E_OIDC_ISSUER = 'http://keycloak:18080/realms/citadel-e2e'
        & npx playwright test @TestFiles --project=chromium
        if ($LASTEXITCODE -ne 0) { throw 'The identity Rust frontend browser suite failed.' }
    }
    finally {
        Remove-Item Env:CITADEL_E2E_BASE_URL -ErrorAction SilentlyContinue
        Remove-Item Env:CITADEL_E2E_OIDC_ISSUER -ErrorAction SilentlyContinue
        Pop-Location
    }
}
finally {
    $containers = @(& docker ps --all --format '{{.Names}}')
    foreach ($container in @($server, $keycloak, $postgres)) {
        if ($containers -contains $container) {
            & docker rm --force $container | Out-Null
        }
    }
    if (@(& docker network ls --format '{{.Name}}') -contains $network) {
        & docker network rm $network | Out-Null
    }
    if ($removeBuildCache) { Remove-CitadelBuildCache -Name $BuildCacheVolume }
}
