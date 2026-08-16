param(
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$suffix = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$network = "citadel-rust-phase3-$suffix"
$postgres = "citadel-rust-phase3-postgres-$suffix"
$server = "citadel-rust-phase3-server-$suffix"
$rustImage = 'rust@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97'
$serverDatabase = 'citadel_phase3_server'
$integrationDatabase = 'citadel_phase3_integration'

function Wait-ForServer {
    param([string]$Origin)

    $lastReady = 'not requested'
    foreach ($attempt in 1..60) {
        try {
            $health = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$Origin/health"
            $ready = Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri "$Origin/ready"
            $lastReady = "$($ready.StatusCode) $($ready.Content)"
            if ($health.StatusCode -eq 200 -and $ready.StatusCode -eq 200) {
                return
            }
        }
        catch {
            if ($_.ErrorDetails.Message) {
                $lastReady = $_.ErrorDetails.Message
            }
            elseif ($_.Exception.Message) {
                $lastReady = $_.Exception.Message
            }
        }
        Start-Sleep -Milliseconds 500
    }
    & docker logs $server
    throw "The Phase 3 server did not become ready. Last response: $lastReady"
}

function Start-Phase3Server {
    & docker run --detach --name $server --network $network `
        --publish '127.0.0.1::8000' `
        --volume "${repoRoot}:/source:ro" `
        --volume 'citadel-rust-target:/source/rust/target:ro' `
        --volume '/var/run/docker.sock:/var/run/docker.sock:ro' `
        --workdir /source/rust `
        --env "DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$serverDatabase" `
        --env 'Transport__Mode=Disabled' `
        --env 'Jwt__Key=phase3-only-jwt-key-at-least-32-bytes' `
        --env 'Secrets__EncryptionKey=AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=' `
        --env 'JobConfiguration__MonitoringInterval=1' `
        $rustImage `
        /source/rust/target/debug/citadel-server serve | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 server did not start.' }

    $publishedAddress = & docker port $server '8000/tcp'
    $serverPort = ($publishedAddress -split ':')[-1]
    $script:serverOrigin = "http://127.0.0.1:$serverPort"
    Wait-ForServer $script:serverOrigin
}

try {
    & docker network create $network | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the Phase 3 test network.' }

    & docker run --detach --name $postgres --network $network `
        --env 'POSTGRES_USER=citadel_phase3' `
        --env 'POSTGRES_PASSWORD=citadel_phase3' `
        --env 'POSTGRES_DB=postgres' `
        --health-cmd 'pg_isready -U citadel_phase3 -d postgres' `
        --health-interval 1s --health-timeout 2s --health-retries 30 `
        $PostgresImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not start the Phase 3 PostgreSQL fixture.' }

    $healthy = $false
    foreach ($attempt in 1..40) {
        if ((& docker inspect $postgres --format '{{.State.Health.Status}}') -eq 'healthy') {
            $healthy = $true
            break
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $healthy) { throw 'The Phase 3 PostgreSQL fixture did not become healthy.' }

    foreach ($database in @($serverDatabase, $integrationDatabase)) {
        & docker exec $postgres createdb --username citadel_phase3 --owner citadel_phase3 $database
        if ($LASTEXITCODE -ne 0) { throw "Could not create Phase 3 database '$database'." }
    }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume 'citadel-rust-target:/source/rust/target' `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-adapters --test identity_access -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The persisted Phase 3 identity/ACL integration test failed.' }

    & docker run --rm `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume 'citadel-rust-target:/source/rust/target' `
        --workdir /source/rust `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo build --locked -p citadel-server
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 server did not build.' }

    Start-Phase3Server

    $setup = Invoke-RestMethod -Method Get -Uri "$serverOrigin/api/v1/setup/status"
    if (-not $setup.requiresSetup) { throw 'A clean Phase 3 database did not require setup.' }

    $browser = New-Object Microsoft.PowerShell.Commands.WebRequestSession
    $initializeBody = @{
        name = 'owner'
        email = 'owner@example.test'
        password = 'correct-horse-battery-staple'
    } | ConvertTo-Json
    $initialize = Invoke-WebRequest -UseBasicParsing -Method Post `
        -Uri "$serverOrigin/api/v1/setup/initialize" `
        -ContentType 'application/json' -Body $initializeBody -WebSession $browser
    $initializeResult = $initialize.Content | ConvertFrom-Json
    if ($initialize.StatusCode -ne 200 -or [string]::IsNullOrWhiteSpace($initializeResult.accessToken)) {
        throw 'Initial administrator setup did not issue an access token.'
    }
    if ($browser.Cookies.Count -ne 1) {
        throw 'Disabled HTTP transport did not retain the refresh cookie for local development.'
    }

    $matrix = Invoke-WebRequest -UseBasicParsing -Uri "$serverOrigin/api/v1/roles/permissions/matrix"
    if ($matrix.StatusCode -ne 200 -or $matrix.Content -notmatch 'ServiceAccount') {
        throw 'The anonymous permission matrix is unavailable or incomplete.'
    }

    $authorization = @{ Authorization = "Bearer $($initializeResult.accessToken)" }
    $accounts = Invoke-WebRequest -UseBasicParsing -Uri "$serverOrigin/api/v1/serviceAccounts" -Headers $authorization
    if ($accounts.StatusCode -ne 200 -or $accounts.Content -notmatch 'pagedResult') {
        throw 'The administrator could not list Service Accounts.'
    }

    $createBody = @{ name = 'ci'; description = 'fixture'; isEnabled = $true } | ConvertTo-Json
    try {
        Invoke-WebRequest -UseBasicParsing -Method Post `
            -Uri "$serverOrigin/api/v1/serviceAccounts" -Headers $authorization `
            -ContentType 'application/json' -Body $createBody | Out-Null
        throw 'Service Account creation bypassed the Custom Access Control license gate.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 403) { throw }
    }

    $refresh = Invoke-WebRequest -UseBasicParsing -Method Get `
        -Uri "$serverOrigin/api/v1/authentication/refresh" -WebSession $browser
    if ($refresh.StatusCode -ne 200 -or $refresh.Content -notmatch 'accessToken') {
        throw 'The refresh session did not issue a new access token.'
    }

    try {
        Invoke-WebRequest -UseBasicParsing -Method Post `
            -Uri "$serverOrigin/api/v1/authentication/login" `
            -ContentType 'application/json' `
            -Body (@{ emailOrName = 'missing'; password = 'incorrect-password-value' } | ConvertTo-Json) | Out-Null
        throw 'Invalid credentials were accepted.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
    }

    & docker stop --time 15 $server | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 server did not stop gracefully.' }
    if ([int](& docker inspect $server --format '{{.State.ExitCode}}') -ne 0) {
        & docker logs $server
        throw 'The Phase 3 server returned a non-zero exit code during shutdown.'
    }
    & docker rm $server | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'The stopped Phase 3 server could not be removed.' }
    Start-Phase3Server

    $setupAfterRestart = Invoke-RestMethod -Method Get -Uri "$serverOrigin/api/v1/setup/status"
    if ($setupAfterRestart.requiresSetup) { throw 'Setup state was not preserved across restart.' }
    $login = Invoke-RestMethod -Method Post `
        -Uri "$serverOrigin/api/v1/authentication/login" `
        -ContentType 'application/json' `
        -Body (@{ emailOrName = 'owner@example.test'; password = 'correct-horse-battery-staple' } | ConvertTo-Json)
    if ([string]::IsNullOrWhiteSpace($login.accessToken)) {
        throw 'The persisted administrator could not sign in after restart.'
    }

    $password = & docker exec $postgres psql --username citadel_phase3 --dbname $serverDatabase `
        --tuples-only --no-align --command "SELECT password FROM users WHERE email = 'owner@example.test'"
    if ($password -notlike 'cit_pwd_v1$*' -or $password -match 'correct-horse') {
        throw 'The persisted password is not using the versioned Argon2id envelope.'
    }
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
}
