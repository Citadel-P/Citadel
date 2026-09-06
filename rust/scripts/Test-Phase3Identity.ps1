param(
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')
$buildCacheVolume = New-CitadelBuildCacheName
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
        --volume "${buildCacheVolume}:/source/rust/target:ro" `
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

    Push-Location $repoRoot
    try {
        & dotnet run --project 'test/Citadel.Tests.Integration/Citadel.Tests.Integration.csproj' `
            -- -class 'Tests.Integration.Application.Features.Permissions.AuthorizationDifferentialTests'
        if ($LASTEXITCODE -ne 0) { throw 'The .NET authorization differential oracle failed.' }
    }
    finally {
        Pop-Location
    }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-adapters --test identity_access -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The persisted Phase 3 identity/ACL integration test failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-adapters --test identity_authorization_differential -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The Rust authorization differential matrix failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-server --test users_http -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 User HTTP integration tests failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-server --test teams_http -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 Team HTTP integration tests failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-server --test roles_http -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 Role HTTP integration tests failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-server --test licenses_http -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 License HTTP integration tests failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-server --test mfa_http -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 MFA HTTP integration tests failed.' }

    & docker run --rm --network $network `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env "CITADEL_PHASE3_DATABASE_URL=postgres://citadel_phase3:citadel_phase3@${postgres}:5432/$integrationDatabase" `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-server --test oidc_http -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 OIDC HTTP integration tests failed.' }

    & docker run --rm `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo test --locked -p citadel-adapters --test oidc_protocol
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 OIDC protocol compatibility test failed.' }

    & docker run --rm `
        --volume "${repoRoot}:/source" `
        --volume 'citadel-rust-registry:/usr/local/cargo/registry' `
        --volume 'citadel-rust-git:/usr/local/cargo/git' `
        --volume 'citadel-rustup:/usr/local/rustup' `
        --volume "${buildCacheVolume}:/source/rust/target" `
        --workdir /source/rust `
        --env 'SQLX_OFFLINE=true' `
        $rustImage `
        cargo build --locked -p citadel-server
    if ($LASTEXITCODE -ne 0) { throw 'The Phase 3 server did not build.' }

    Start-Phase3Server

    $setup = Invoke-RestMethod -Method Get -Uri "$serverOrigin/api/v1/setup/status"
    if (-not $setup.requiresSetup) { throw 'A clean Phase 3 database did not require setup.' }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/application/info" | Out-Null
        throw 'Application information bypassed the first-run setup gate.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 409) { throw }
    }

    $browser = New-Object Microsoft.PowerShell.Commands.WebRequestSession
    $initializeBody = @{
        name = 'owner'
        email = 'owner@example.test'
        password = 'correct-horse-battery-staple'
    } | ConvertTo-Json
    $initialize = Invoke-WebRequest -UseBasicParsing -Method Post `
        -Uri "$serverOrigin/api/v1/setup/initialize" `
        -ContentType 'application/json' -Body $initializeBody -WebSession $browser `
        -UserAgent 'Mozilla/5.0 (Windows NT 10.0) AppleWebKit Chrome/140.0 Safari/537.36'
    $initializeResult = $initialize.Content | ConvertFrom-Json
    if ($initialize.StatusCode -ne 200 -or [string]::IsNullOrWhiteSpace($initializeResult.accessToken)) {
        throw 'Initial administrator setup did not issue an access token.'
    }
    if ($browser.Cookies.Count -ne 1) {
        throw 'Disabled HTTP transport did not retain the refresh cookie for local development.'
    }

    $matrixResponse = Invoke-WebRequest -UseBasicParsing -Uri "$serverOrigin/api/v1/roles/permissions/matrix"
    $matrix = $matrixResponse.Content | ConvertFrom-Json
    if ($matrixResponse.StatusCode -ne 200 -or $null -eq $matrix.ServiceAccount -or `
        $matrix.Deployment.maximumLevel -ne 'Execute' -or `
        $matrix.Deployment.specificPermissions.Apply -ne 'Read') {
        throw 'The anonymous permission matrix is unavailable or incomplete.'
    }

    $authorization = @{ Authorization = "Bearer $($initializeResult.accessToken)" }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/application/info" | Out-Null
        throw 'Application information was returned without authentication.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
    }
    $applicationInfoBeforeRestart = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/application/info" -Headers $authorization
    $expectedDisplayVersion = ($applicationInfoBeforeRestart.informationalVersion -split '\+', 2)[0]
    if ($applicationInfoBeforeRestart.name -ne 'Citadel' -or `
        [string]::IsNullOrWhiteSpace($applicationInfoBeforeRestart.informationalVersion) -or `
        $applicationInfoBeforeRestart.version -ne $expectedDisplayVersion -or `
        $applicationInfoBeforeRestart.realtimeTransport -ne 'WebSocketV1') {
        throw 'Application information did not use the embedded build metadata.'
    }
    $licenseEntitlements = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/license/entitlements" -Headers $authorization
    if ($licenseEntitlements.status -ne 'Community' -or `
        @($licenseEntitlements.capabilities).Count -ne 5) {
        throw 'The authenticated Community entitlement projection is incomplete.'
    }
    $licenseRequestBeforeRestart = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/license/request" -Headers $authorization
    if ($licenseRequestBeforeRestart.product -ne 'citadel' -or `
        [string]::IsNullOrWhiteSpace($licenseRequestBeforeRestart.instanceId) -or `
        $licenseRequestBeforeRestart.coreVersion -ne $applicationInfoBeforeRestart.version) {
        throw 'License request metadata did not use stable instance and Core version values.'
    }
    $profile = Invoke-RestMethod -Method Get -Uri "$serverOrigin/api/v1/profile" -Headers $authorization
    if ($profile.displayName -ne 'owner' -or $profile.email -ne 'owner@example.test') {
        throw 'The initialized administrator profile was not returned.'
    }
    $profile = Invoke-RestMethod -Method Patch -Uri "$serverOrigin/api/v1/profile" `
        -Headers $authorization -ContentType 'application/json' `
        -Body (@{ displayName = 'owner-renamed' } | ConvertTo-Json)
    if ($profile.displayName -ne 'owner-renamed' -or -not $profile.authorization.isAdministrator) {
        throw 'The current profile update was not persisted or lost authorization metadata.'
    }
    $preferences = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/profile/preferences" -Headers $authorization
    if ($preferences.isPersisted -or $null -ne $preferences.timeZone -or `
        $preferences.dateTimeFormat -ne 'System' -or $preferences.theme -ne 'System') {
        throw 'Unsaved profile preferences did not return the browser-local defaults.'
    }
    $preferences = Invoke-RestMethod -Method Patch `
        -Uri "$serverOrigin/api/v1/profile/preferences" -Headers $authorization `
        -ContentType 'application/merge-patch+json' `
        -Body (@{
            timeZone = 'Europe/Paris'
            dateTimeFormat = 'TwentyFourHour'
            theme = 'Dark'
        } | ConvertTo-Json)
    if (-not $preferences.isPersisted -or $preferences.timeZone -ne 'Europe/Paris' -or `
        $preferences.dateTimeFormat -ne 'TwentyFourHour' -or $preferences.theme -ne 'Dark') {
        throw 'Profile preferences were not validated and persisted.'
    }
    foreach ($invalidPreferenceBody in @(
        (@{ timeZone = 'Not/AZone' } | ConvertTo-Json),
        (@{ timeZone = $null } | ConvertTo-Json),
        '{}'
    )) {
        try {
            Invoke-WebRequest -UseBasicParsing -Method Patch `
                -Uri "$serverOrigin/api/v1/profile/preferences" -Headers $authorization `
                -ContentType 'application/merge-patch+json' -Body $invalidPreferenceBody | Out-Null
            throw 'An invalid profile preference patch was accepted.'
        }
        catch {
            if ($_.Exception.Response.StatusCode.value__ -ne 400) { throw }
        }
    }

    $secondaryBrowser = New-Object Microsoft.PowerShell.Commands.WebRequestSession
    $secondaryLogin = Invoke-WebRequest -UseBasicParsing -Method Post `
        -Uri "$serverOrigin/api/v1/authentication/login" `
        -ContentType 'application/json' -WebSession $secondaryBrowser `
        -UserAgent 'Firefox/140.0 (Linux)' `
        -Body (@{
            emailOrName = 'owner@example.test'
            password = 'correct-horse-battery-staple'
        } | ConvertTo-Json)
    if ($secondaryLogin.StatusCode -ne 200 -or $secondaryBrowser.Cookies.Count -ne 1) {
        throw 'A secondary browser session could not be created.'
    }
    $sessions = Invoke-RestMethod -Method Get -Uri "$serverOrigin/api/v1/profile/sessions" `
        -Headers $authorization -WebSession $browser
    if (-not $sessions.canRevokeOtherSessions -or $sessions.sessions.Count -ne 2) {
        throw 'Active profile sessions were not listed with current-session capabilities.'
    }
    $currentSession = @($sessions.sessions | Where-Object isCurrent)
    $otherSessions = @($sessions.sessions | Where-Object { -not $_.isCurrent })
    if ($currentSession.Count -ne 1 -or $otherSessions.Count -ne 1 -or `
        $currentSession[0].displayName -ne 'Chrome on Windows') {
        throw 'The current profile session was not identified or ordered correctly.'
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Delete `
            -Uri "$serverOrigin/api/v1/profile/sessions/$($currentSession[0].id)" `
            -Headers $authorization -WebSession $browser | Out-Null
        throw 'The current profile session was revoked through the individual endpoint.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 404) { throw }
    }
    $individualRevoke = Invoke-WebRequest -UseBasicParsing -Method Delete `
        -Uri "$serverOrigin/api/v1/profile/sessions/$($otherSessions[0].id)" `
        -Headers $authorization -WebSession $browser
    if ($individualRevoke.StatusCode -ne 204) {
        throw 'An owned secondary profile session was not revoked.'
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/authentication/refresh" `
            -WebSession $secondaryBrowser | Out-Null
        throw 'An individually revoked secondary session could still refresh.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
    }

    $otherBrowsers = @()
    foreach ($userAgent in @('Edg/140.0 (Mac OS X)', 'curl/8.16.0')) {
        $otherBrowser = New-Object Microsoft.PowerShell.Commands.WebRequestSession
        $otherLogin = Invoke-WebRequest -UseBasicParsing -Method Post `
            -Uri "$serverOrigin/api/v1/authentication/login" `
            -ContentType 'application/json' -WebSession $otherBrowser -UserAgent $userAgent `
            -Body (@{
                emailOrName = 'owner@example.test'
                password = 'correct-horse-battery-staple'
            } | ConvertTo-Json)
        if ($otherLogin.StatusCode -ne 200 -or $otherBrowser.Cookies.Count -ne 1) {
            throw 'An additional browser session could not be created.'
        }
        $otherBrowsers += $otherBrowser
    }
    $revokeOthers = Invoke-RestMethod -Method Delete `
        -Uri "$serverOrigin/api/v1/profile/sessions" `
        -Headers $authorization -WebSession $browser
    if ($revokeOthers.count -ne 2) {
        throw 'The other profile sessions were not revoked atomically.'
    }
    $sessions = Invoke-RestMethod -Method Get -Uri "$serverOrigin/api/v1/profile/sessions" `
        -Headers $authorization -WebSession $browser
    if ($sessions.sessions.Count -ne 1 -or -not $sessions.sessions[0].isCurrent) {
        throw 'Revoking other sessions removed or misidentified the current session.'
    }
    foreach ($otherBrowser in $otherBrowsers) {
        try {
            Invoke-WebRequest -UseBasicParsing -Method Get `
                -Uri "$serverOrigin/api/v1/authentication/refresh" `
                -WebSession $otherBrowser | Out-Null
            throw 'A revoked secondary session could still refresh.'
        }
        catch {
            if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
        }
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Delete `
            -Uri "$serverOrigin/api/v1/profile/sessions" -Headers $authorization `
            -WebSession (New-Object Microsoft.PowerShell.Commands.WebRequestSession) | Out-Null
        throw 'Other sessions were revoked without a current refresh session.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 400) { throw }
    }

    $passwordOtherBrowser = New-Object Microsoft.PowerShell.Commands.WebRequestSession
    $passwordOtherLogin = Invoke-WebRequest -UseBasicParsing -Method Post `
        -Uri "$serverOrigin/api/v1/authentication/login" `
        -ContentType 'application/json' -WebSession $passwordOtherBrowser `
        -UserAgent 'Password change secondary session' `
        -Body (@{
            emailOrName = 'owner@example.test'
            password = 'correct-horse-battery-staple'
        } | ConvertTo-Json)
    if ($passwordOtherLogin.StatusCode -ne 200 -or $passwordOtherBrowser.Cookies.Count -ne 1) {
        throw 'The password-change secondary session could not be created.'
    }
    foreach ($invalidPasswordChange in @(
        @{
            currentPassword = 'wrong-current-password'
            newPassword = 'new-correct-horse-battery-staple'
        },
        @{
            currentPassword = 'correct-horse-battery-staple'
            newPassword = 'short'
        }
    )) {
        try {
            Invoke-WebRequest -UseBasicParsing -Method Post `
                -Uri "$serverOrigin/api/v1/profile/change-password" `
                -Headers $authorization -WebSession $browser `
                -ContentType 'application/json' `
                -Body ($invalidPasswordChange | ConvertTo-Json) | Out-Null
            throw 'An invalid password change was accepted.'
        }
        catch {
            if ($_.Exception.Response.StatusCode.value__ -ne 400) { throw }
        }
    }
    $passwordChange = Invoke-WebRequest -UseBasicParsing -Method Post `
        -Uri "$serverOrigin/api/v1/profile/change-password" `
        -Headers $authorization -WebSession $browser `
        -ContentType 'application/json' `
        -Body (@{
            currentPassword = 'correct-horse-battery-staple'
            newPassword = 'new-correct-horse-battery-staple'
        } | ConvertTo-Json)
    if ($passwordChange.StatusCode -ne 204) {
        throw 'A valid password change did not return No Content.'
    }
    $currentRefresh = Invoke-WebRequest -UseBasicParsing -Method Get `
        -Uri "$serverOrigin/api/v1/authentication/refresh" -WebSession $browser
    if ($currentRefresh.StatusCode -ne 200) {
        throw 'Changing the password revoked the current browser session.'
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/authentication/refresh" `
            -WebSession $passwordOtherBrowser | Out-Null
        throw 'Changing the password did not revoke another browser session.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Post `
            -Uri "$serverOrigin/api/v1/authentication/login" `
            -ContentType 'application/json' `
            -Body (@{
                emailOrName = 'owner@example.test'
                password = 'correct-horse-battery-staple'
            } | ConvertTo-Json) | Out-Null
        throw 'The previous password remained valid after the password change.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
    }
    $newPasswordLogin = Invoke-WebRequest -UseBasicParsing -Method Post `
        -Uri "$serverOrigin/api/v1/authentication/login" `
        -ContentType 'application/json' `
        -Body (@{
            emailOrName = 'owner@example.test'
            password = 'new-correct-horse-battery-staple'
        } | ConvertTo-Json)
    if ($newPasswordLogin.StatusCode -ne 200) {
        throw 'The new password could not authenticate.'
    }

    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/activities" | Out-Null
        throw 'Activity history was returned without authentication.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
    }
    $activityPage = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$($profile.id)&resourceType=User&page=1&pageSize=100" `
        -Headers $authorization
    $profileActivities = @($activityPage.pagedResult.items)
    foreach ($eventType in @(
        'UserProfileUpdated',
        'UserPreferencesUpdated',
        'UserPasswordChanged',
        'UserSessionRevoked',
        'UserOtherSessionsRevoked'
    )) {
        if ($profileActivities.eventType -notcontains $eventType) {
            throw "Profile activity history is missing '$eventType'."
        }
    }
    $activityJson = $activityPage | ConvertTo-Json -Depth 20 -Compress
    if ($activityJson -match 'correct-horse-battery-staple') {
        throw 'Profile activity history exposed password material.'
    }
    $activityDetail = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities/$($profileActivities[0].id)" `
        -Headers $authorization
    if ($activityDetail.id -ne $profileActivities[0].id -or `
        [string]::IsNullOrWhiteSpace($activityDetail.info.'$type')) {
        throw 'Activity detail did not preserve the existing frontend contract.'
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/activities?pageSize=501" `
            -Headers $authorization | Out-Null
        throw 'The bounded Activity page size was not enforced.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 400) { throw }
    }

    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/users?Page=1&PageSize=10" | Out-Null
        throw 'The administrator User list was returned without authentication.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 401) { throw }
    }
    $users = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/users?Page=1&PageSize=10&Name=owner" `
        -Headers $authorization
    $userItems = @($users.pagedResult.items)
    if ($users.pagedResult.totalCount -ne 1 -or $userItems.Count -ne 1 -or `
        $userItems[0].id -ne $profile.id -or $userItems[0].name -ne 'owner-renamed' -or `
        -not $users.capabilities.canRead) {
        throw 'The administrator User list did not preserve paging, filtering, or capabilities.'
    }
    $searchedUsers = @(Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/users/search?Query=OWNER%40EXAMPLE&Limit=10" `
        -Headers $authorization)
    if ($searchedUsers.Count -ne 1 -or $searchedUsers[0].id -ne $profile.id) {
        throw 'The administrator User search did not match email case-insensitively.'
    }
    $userDetail = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/users/$($profile.id)" -Headers $authorization
    if ($userDetail.name -ne 'owner-renamed' -or $userDetail.email -ne 'owner@example.test' -or `
        -not $userDetail.isEnabled -or $null -eq $userDetail.resourceAccesses) {
        throw 'The administrator User detail did not include the expected identity and ACL shape.'
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/users/search?Query=x" -Headers $authorization | Out-Null
        throw 'A one-character User search query was accepted.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 400) { throw }
    }

    $managedUserName = "phase3-user-$suffix"
    $managedUser = Invoke-RestMethod -Method Post `
        -Uri "$serverOrigin/api/v1/users" -Headers $authorization `
        -ContentType 'application/json' `
        -Body (@{
            name = $managedUserName
            email = "$managedUserName@example.test"
            password = 'managed-user-correct-horse-battery-staple'
            isEnabled = $true
        } | ConvertTo-Json)
    if ($managedUser.name -ne $managedUserName -or -not $managedUser.isEnabled) {
        throw 'The administrator User create mutation did not return the persisted User.'
    }
    $managedUser = Invoke-RestMethod -Method Patch `
        -Uri "$serverOrigin/api/v1/users/$($managedUser.id)" -Headers $authorization `
        -ContentType 'application/merge-patch+json' `
        -Body (@{ email = "updated-$managedUserName@example.test" } | ConvertTo-Json)
    if ($managedUser.email -ne "updated-$managedUserName@example.test") {
        throw 'The administrator User patch mutation was not persisted.'
    }
    $renamedManagedUser = "$managedUserName-renamed"
    $managedUser = Invoke-RestMethod -Method Post `
        -Uri "$serverOrigin/api/v1/users/rename" -Headers $authorization `
        -ContentType 'application/json' `
        -Body (@{ id = $managedUser.id; name = $renamedManagedUser } | ConvertTo-Json)
    if ($managedUser.name -ne $renamedManagedUser) {
        throw 'The administrator User rename mutation was not persisted.'
    }
    $managedUserActivities = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$($managedUser.id)&resourceType=User&pageSize=100" `
        -Headers $authorization
    foreach ($eventType in @('UserCreated', 'UserUpdated', 'UserRenamed')) {
        if (@($managedUserActivities.pagedResult.items).eventType -notcontains $eventType) {
            throw "Administrator User activity history is missing '$eventType'."
        }
    }

    $managedTeamName = "phase3-team-$suffix"
    $managedTeam = Invoke-RestMethod -Method Post `
        -Uri "$serverOrigin/api/v1/teams" -Headers $authorization `
        -ContentType 'application/json' `
        -Body (@{ name = $managedTeamName; userIds = @($managedUser.id) } | ConvertTo-Json)
    if ($managedTeam.name -ne $managedTeamName -or $managedTeam.totalMembers -ne 1) {
        throw 'The administrator Team create mutation did not persist its member.'
    }
    $managedTeam = Invoke-RestMethod -Method Patch `
        -Uri "$serverOrigin/api/v1/teams/$($managedTeam.id)" -Headers $authorization `
        -ContentType 'application/merge-patch+json' `
        -Body (@{ isEnabled = $false } | ConvertTo-Json)
    if ($managedTeam.isEnabled) {
        throw 'The administrator Team patch mutation was not persisted.'
    }
    $renamedManagedTeam = "$managedTeamName-renamed"
    $managedTeam = Invoke-RestMethod -Method Post `
        -Uri "$serverOrigin/api/v1/teams/rename" -Headers $authorization `
        -ContentType 'application/json' `
        -Body (@{ id = $managedTeam.id; name = $renamedManagedTeam } | ConvertTo-Json)
    if ($managedTeam.name -ne $renamedManagedTeam) {
        throw 'The administrator Team rename mutation was not persisted.'
    }
    $managedTeamActivities = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$($managedTeam.id)&resourceType=Team&pageSize=100" `
        -Headers $authorization
    foreach ($eventType in @('TeamCreated', 'TeamUpdated', 'TeamRenamed')) {
        if (@($managedTeamActivities.pagedResult.items).eventType -notcontains $eventType) {
            throw "Administrator Team activity history is missing '$eventType'."
        }
    }

    $managedRoleId = [Guid]::NewGuid()
    $managedRolePermissionId = [Guid]::NewGuid()
    $managedRoleName = "phase3-role-$suffix"
    $seedRoleSql = "INSERT INTO roles (id, name, roletype) VALUES ('$managedRoleId', '$managedRoleName', 'Custom'); INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions) VALUES ('$managedRolePermissionId', 4, 3, '$managedRoleId', 0);"
    & docker exec $postgres psql --username citadel_phase3 --dbname $serverDatabase `
        --set ON_ERROR_STOP=1 --command $seedRoleSql | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not seed the unlicensed Role reduction fixture.' }
    $managedRole = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/roles/$managedRoleId" -Headers $authorization
    if ($managedRole.name -ne $managedRoleName -or `
        $managedRole.permissions[0].permissionLevel -ne 'Execute') {
        throw 'The administrator Role detail projection was not returned.'
    }
    $managedRole = Invoke-RestMethod -Method Patch `
        -Uri "$serverOrigin/api/v1/roles/$managedRoleId/permissions" -Headers $authorization `
        -ContentType 'application/merge-patch+json' `
        -Body (@{ permissions = @(@{
            resourceType = 'Registry'
            permissionLevel = 'Read'
            specificPermissions = @()
        }) } | ConvertTo-Json -Depth 5)
    if ($managedRole.permissions.Count -ne 1 -or `
        $managedRole.permissions[0].permissionLevel -ne 'Read') {
        throw 'A safe Role permission reduction was blocked after license downgrade.'
    }
    $renamedManagedRole = "$managedRoleName-renamed"
    $managedRole = Invoke-RestMethod -Method Post `
        -Uri "$serverOrigin/api/v1/roles/rename" -Headers $authorization `
        -ContentType 'application/json' `
        -Body (@{ id = $managedRoleId; name = $renamedManagedRole } | ConvertTo-Json)
    if ($managedRole.name -ne $renamedManagedRole) {
        throw 'The administrator Role rename mutation was not persisted.'
    }
    $managedRoleActivities = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$managedRoleId&resourceType=Role&pageSize=100" `
        -Headers $authorization
    foreach ($eventType in @('RoleUpdated', 'RoleRenamed')) {
        if (@($managedRoleActivities.pagedResult.items).eventType -notcontains $eventType) {
            throw "Administrator Role activity history is missing '$eventType'."
        }
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Post `
            -Uri "$serverOrigin/api/v1/roles" -Headers $authorization `
            -ContentType 'application/json' `
            -Body (@{
                name = "unlicensed-role-$suffix"
                permissions = @(@{
                    resourceType = 'Registry'
                    permissionLevel = 'Read'
                    specificPermissions = @()
                })
            } | ConvertTo-Json -Depth 5) | Out-Null
        throw 'Role creation bypassed the Custom Access Control license gate.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 403) { throw }
    }

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
        -Body (@{ emailOrName = 'owner@example.test'; password = 'new-correct-horse-battery-staple' } | ConvertTo-Json)
    if ([string]::IsNullOrWhiteSpace($login.accessToken)) {
        throw 'The persisted administrator could not sign in after restart.'
    }
    $applicationInfoAfterRestart = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/application/info" `
        -Headers @{ Authorization = "Bearer $($login.accessToken)" }
    if ($applicationInfoAfterRestart.name -ne $applicationInfoBeforeRestart.name -or `
        $applicationInfoAfterRestart.version -ne $applicationInfoBeforeRestart.version -or `
        $applicationInfoAfterRestart.informationalVersion -ne $applicationInfoBeforeRestart.informationalVersion -or `
        $applicationInfoAfterRestart.realtimeTransport -ne $applicationInfoBeforeRestart.realtimeTransport) {
        throw 'Application build information changed after restart.'
    }
    $licenseRequestAfterRestart = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/license/request" `
        -Headers @{ Authorization = "Bearer $($login.accessToken)" }
    if ($licenseRequestAfterRestart.instanceId -ne $licenseRequestBeforeRestart.instanceId) {
        throw 'The Citadel license instance identity changed after restart.'
    }
    $persistedProfile = Invoke-RestMethod -Method Get -Uri "$serverOrigin/api/v1/profile" `
        -Headers @{ Authorization = "Bearer $($login.accessToken)" }
    if ($persistedProfile.displayName -ne 'owner-renamed') {
        throw 'The profile update was not preserved across restart.'
    }
    $persistedPreferences = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/profile/preferences" `
        -Headers @{ Authorization = "Bearer $($login.accessToken)" }
    if (-not $persistedPreferences.isPersisted -or `
        $persistedPreferences.timeZone -ne 'Europe/Paris' -or `
        $persistedPreferences.dateTimeFormat -ne 'TwentyFourHour' -or `
        $persistedPreferences.theme -ne 'Dark') {
        throw 'Profile preferences were not preserved across restart.'
    }
    $persistedActivities = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$($profile.id)&resourceType=User&pageSize=100" `
        -Headers @{ Authorization = "Bearer $($login.accessToken)" }
    if ($persistedActivities.pagedResult.totalCount -lt $profileActivities.Count) {
        throw 'Profile activity history was not preserved across restart.'
    }
    $persistedUsers = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/users?Page=1&PageSize=10&Name=owner" `
        -Headers @{ Authorization = "Bearer $($login.accessToken)" }
    if ($persistedUsers.pagedResult.totalCount -ne 1 -or `
        @($persistedUsers.pagedResult.items)[0].name -ne 'owner-renamed') {
        throw 'The administrator User projection was not preserved across restart.'
    }
    $restartAuthorization = @{ Authorization = "Bearer $($login.accessToken)" }
    $persistedManagedUser = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/users/$($managedUser.id)" -Headers $restartAuthorization
    if ($persistedManagedUser.name -ne $renamedManagedUser -or `
        $persistedManagedUser.email -ne "updated-$managedUserName@example.test") {
        throw 'Administrator User mutations were not preserved across restart.'
    }
    $persistedManagedTeam = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/teams/$($managedTeam.id)" -Headers $restartAuthorization
    if ($persistedManagedTeam.name -ne $renamedManagedTeam -or `
        $persistedManagedTeam.isEnabled -or $persistedManagedTeam.totalMembers -ne 1) {
        throw 'Administrator Team mutations were not preserved across restart.'
    }
    $persistedManagedRole = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/roles/$managedRoleId" -Headers $restartAuthorization
    if ($persistedManagedRole.name -ne $renamedManagedRole -or `
        $persistedManagedRole.permissions.Count -ne 1 -or `
        $persistedManagedRole.permissions[0].permissionLevel -ne 'Read') {
        throw 'Administrator Role mutations were not preserved across restart.'
    }
    $deleteManagedRole = Invoke-WebRequest -UseBasicParsing -Method Delete `
        -Uri "$serverOrigin/api/v1/roles" -Headers $restartAuthorization `
        -ContentType 'application/json' `
        -Body (@{ ids = @($managedRoleId) } | ConvertTo-Json)
    if ($deleteManagedRole.StatusCode -ne 204) {
        throw 'The administrator Role delete mutation did not return No Content.'
    }
    $deletedRoleActivities = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$managedRoleId&resourceType=Role&pageSize=100" `
        -Headers $restartAuthorization
    if (@($deletedRoleActivities.pagedResult.items).eventType -notcontains 'RoleDeleted') {
        throw 'The deleted Role did not retain its safe lifecycle Activity.'
    }
    $deleteManagedTeam = Invoke-WebRequest -UseBasicParsing -Method Delete `
        -Uri "$serverOrigin/api/v1/teams" -Headers $restartAuthorization `
        -ContentType 'application/json' `
        -Body (@{ ids = @($managedTeam.id) } | ConvertTo-Json)
    if ($deleteManagedTeam.StatusCode -ne 204) {
        throw 'The administrator Team delete mutation did not return No Content.'
    }
    $deletedTeamActivities = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$($managedTeam.id)&resourceType=Team&pageSize=100" `
        -Headers $restartAuthorization
    if (@($deletedTeamActivities.pagedResult.items).eventType -notcontains 'TeamDeleted') {
        throw 'The deleted Team did not retain its safe lifecycle Activity.'
    }
    $deleteManagedUser = Invoke-WebRequest -UseBasicParsing -Method Delete `
        -Uri "$serverOrigin/api/v1/users" -Headers $restartAuthorization `
        -ContentType 'application/json' `
        -Body (@{ ids = @($managedUser.id) } | ConvertTo-Json)
    if ($deleteManagedUser.StatusCode -ne 204) {
        throw 'The administrator User delete mutation did not return No Content.'
    }
    try {
        Invoke-WebRequest -UseBasicParsing -Method Get `
            -Uri "$serverOrigin/api/v1/users/$($managedUser.id)" `
            -Headers $restartAuthorization | Out-Null
        throw 'The deleted User remained readable.'
    }
    catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 404) { throw }
    }
    $deletedUserActivities = Invoke-RestMethod -Method Get `
        -Uri "$serverOrigin/api/v1/activities?resourceId=$($managedUser.id)&resourceType=User&pageSize=100" `
        -Headers $restartAuthorization
    if (@($deletedUserActivities.pagedResult.items).eventType -notcontains 'UserDeleted') {
        throw 'The deleted User did not retain its safe lifecycle Activity.'
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
    Remove-CitadelBuildCache -Name $buildCacheVolume
}
