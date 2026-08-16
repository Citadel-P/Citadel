param(
    [switch]$SkipImageBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$toolImage = 'citadel-drizzle:0.1.15'
$postgresImage = 'postgres@sha256:aa6eb304ddb6dd26df23d05db4e5cb05af8951cda3e0dc57731b771e0ef4ab29'
$network = 'citadel-rust-phase1-drizzle'
$databaseContainer = 'citadel-rust-phase1-drizzle-db'
$temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$temporaryDirectory = Join-Path $temporaryRoot ('citadel-phase1-drizzle-' + [Guid]::NewGuid().ToString('N'))
$password = 'phase1-drizzle-proof'

function Invoke-Docker([string[]]$Arguments, [string]$FailureMessage) {
    & docker @Arguments
    if ($LASTEXITCODE -ne 0) { throw $FailureMessage }
}

function Remove-ProofResources {
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & docker rm --force $databaseContainer *> $null
        & docker network rm $network *> $null
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
}

try {
    New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null

    if (-not $SkipImageBuild) {
        Invoke-Docker @(
            'build', '--file', 'rust/schema-tooling/Dockerfile.drizzle',
            '--tag', $toolImage, '.'
        ) 'Building the pinned Drizzle candidate failed.'
    }

    Remove-ProofResources
    Invoke-Docker @('network', 'create', $network) 'Creating the Drizzle proof network failed.'
    Invoke-Docker @(
        'run', '--detach', '--name', $databaseContainer, '--network', $network,
        '--env', "POSTGRES_PASSWORD=$password",
        '--env', 'POSTGRES_DB=citadel',
        $postgresImage
    ) 'Starting the Drizzle proof database failed.'

    $ready = $false
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            $logs = (& docker logs $databaseContainer 2>&1) -join "`n"
            $readyMessages = [regex]::Matches($logs, 'database system is ready to accept connections').Count
            if ($readyMessages -ge 2) {
                & docker exec $databaseContainer psql -U postgres -d citadel -Atc 'SELECT 1;' *> $null
                if ($LASTEXITCODE -eq 0) { $ready = $true; break }
            }
            Start-Sleep -Milliseconds 500
        }
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if (-not $ready) { throw 'The Drizzle proof database did not become ready.' }

    $sourceSql = Join-Path $repoRoot 'src\Citadel.Infrastructure\Scripts\script0001.sql'
    Get-Content $sourceSql -Raw |
        docker exec -i $databaseContainer psql -v ON_ERROR_STOP=1 -U postgres -d citadel *> $null
    if ($LASTEXITCODE -ne 0) { throw 'The accepted .NET development baseline did not apply.' }

    $mountRepo = "type=bind,source=$repoRoot,target=/work,readonly"
    $mountOutput = "type=bind,source=$temporaryDirectory,target=/work/rust/schema-tooling/proof-output"
    $databaseUrl = "postgres://postgres:$password@$databaseContainer`:5432/citadel?sslmode=disable"
    Invoke-Docker @(
        'run', '--rm', '--network', $network,
        '--mount', $mountRepo,
        '--mount', $mountOutput,
        '--workdir', '/work/rust/schema-tooling',
        '--env', "PHASE1_DATABASE_URL=$databaseUrl",
        $toolImage,
        '--config', 'drizzle.config.toml',
        'introspect', '--out', 'proof-output', '--casing', 'preserve',
        '--schemaFilters', 'public'
    ) 'Drizzle could not introspect the complete accepted Citadel schema.'

    $schemaPath = Join-Path $temporaryDirectory 'schema.rs'
    if (-not (Test-Path $schemaPath)) { throw 'Drizzle did not generate a declarative Rust schema.' }
    $schema = Get-Content $schemaPath -Raw
    $tableCount = [regex]::Matches($schema, '#\[PostgresTable').Count
    if ($tableCount -ne 83) {
        throw "Expected all 83 accepted tables in the Rust schema, observed $tableCount."
    }

    $requiredPatterns = [ordered]@{
        defaults = 'default_sql'
        foreignKeys = 'references\s*='
        uniqueIndexes = '#\[PostgresIndex\(unique'
        partialIndexes = 'where\s*='
        checks = 'check\(name\s*='
        json = 'jsonb'
    }
    foreach ($entry in $requiredPatterns.GetEnumerator()) {
        if ($schema -notmatch $entry.Value) {
            throw "The generated declarative schema omitted $($entry.Key)."
        }
    }

    $migrationCountBefore = @(Get-ChildItem $temporaryDirectory -Recurse -File -Filter 'migration.sql').Count
    Invoke-Docker @(
        'run', '--rm', '--network', $network,
        '--mount', $mountRepo,
        '--mount', $mountOutput,
        '--workdir', '/work/rust/schema-tooling',
        '--env', "PHASE1_DATABASE_URL=$databaseUrl",
        $toolImage,
        '--config', 'drizzle.config.toml',
        'generate', '--out', 'proof-output', '--name', 'convergence_check'
    ) 'Drizzle could not diff its generated Rust schema against the introspected snapshot.'
    $migrationCountAfter = @(Get-ChildItem $temporaryDirectory -Recurse -File -Filter 'migration.sql').Count
    if ($migrationCountAfter -ne $migrationCountBefore) {
        throw 'Drizzle did not converge: its generated Rust schema produced another migration.'
    }

    Write-Host 'Drizzle candidate retained all 83 tables, required constructs, and converged without another migration.'
}
finally {
    Remove-ProofResources
    $resolvedTemporaryDirectory = [IO.Path]::GetFullPath($temporaryDirectory)
    if (
        $resolvedTemporaryDirectory.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase) -and
        [IO.Path]::GetFileName($resolvedTemporaryDirectory).StartsWith('citadel-phase1-drizzle-', [StringComparison]::Ordinal)
    ) {
        if (Test-Path $resolvedTemporaryDirectory) {
            Remove-Item -LiteralPath $resolvedTemporaryDirectory -Recurse -Force
        }
    }
    else {
        throw "Refusing to remove unexpected temporary path '$resolvedTemporaryDirectory'."
    }
}
