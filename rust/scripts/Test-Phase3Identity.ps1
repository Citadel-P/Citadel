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
        -ContentType 'application/json' -Body $initializeBody -WebSession $browser `
        -UserAgent 'Mozilla/5.0 (Windows NT 10.0) AppleWebKit Chrome/140.0 Safari/537.36'
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
