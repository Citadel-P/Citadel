# Negative candidate proof: this exits non-zero while the pinned Atlas Community
# release turns declarative rename intent into destructive drop/add SQL.
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$atlasImage = 'citadel-atlas-community:1.3.0'
$postgresImage = 'postgres@sha256:aa6eb304ddb6dd26df23d05db4e5cb05af8951cda3e0dc57731b771e0ef4ab29'
$network = 'citadel-rust-phase1-schema'
$databaseContainer = 'citadel-rust-phase1-schema-db'
$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) ('citadel-phase1-schema-' + [Guid]::NewGuid().ToString('N'))
$password = 'phase1-schema-proof'

function Invoke-Docker([string[]]$Arguments, [string]$FailureMessage) {
    & docker @Arguments
    if ($LASTEXITCODE -ne 0) { throw $FailureMessage }
}

function Invoke-Atlas([string[]]$Arguments) {
    $mountRepo = "type=bind,source=$repoRoot,target=/work,readonly"
    $mountMigrations = "type=bind,source=$temporaryDirectory,target=/migrations"
    Invoke-Docker (@(
        'run', '--rm', '--network', $network,
        '--mount', $mountRepo,
        '--mount', $mountMigrations,
        $atlasImage
    ) + $Arguments) 'Atlas schema command failed.'
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

    Invoke-Docker @(
        'build', '--file', 'rust/schema-tooling/Dockerfile.atlas-community',
        '--tag', $atlasImage, '.'
    ) 'Building the pinned Atlas Community tool failed.'

    Remove-ProofResources
    Invoke-Docker @('network', 'create', $network) 'Creating the schema proof network failed.'
    Invoke-Docker @(
        'run', '--detach', '--name', $databaseContainer, '--network', $network,
        '--env', "POSTGRES_PASSWORD=$password",
        '--env', 'POSTGRES_DB=phase1',
        $postgresImage
    ) 'Starting the schema proof database failed.'

    $ready = $false
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        for ($attempt = 0; $attempt -lt 30; $attempt++) {
            & docker exec $databaseContainer psql -U postgres -d phase1 -Atc 'SELECT 1;' *> $null
            if ($LASTEXITCODE -eq 0) { $ready = $true; break }
            Start-Sleep -Milliseconds 500
        }
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if (-not $ready) { throw 'The schema proof database did not become ready.' }

    foreach ($database in @('dev', 'target', 'current')) {
        Invoke-Docker @('exec', $databaseContainer, 'createdb', '-U', 'postgres', $database) "Creating database '$database' failed."
    }

    $devUrl = "postgres://postgres:$password@$databaseContainer`:5432/dev?sslmode=disable&search_path=public"
    $targetUrl = "postgres://postgres:$password@$databaseContainer`:5432/target?sslmode=disable&search_path=public"
    $currentUrl = "postgres://postgres:$password@$databaseContainer`:5432/current?sslmode=disable&search_path=public"

    Invoke-Atlas @(
        'migrate', 'diff', '0001_proof',
        '--dir', 'file:///migrations',
        '--to', 'file:///work/rust/schema-tooling/proof/schema-v1.sql',
        '--dev-url', $devUrl
    )

    $firstMigration = @(Get-ChildItem $temporaryDirectory -File -Filter '*_0001_proof.sql')[0]
    if (-not $firstMigration) { throw 'Atlas did not generate the initial proof migration.' }
    $firstSql = Get-Content $firstMigration.FullName -Raw
    $requiredPatterns = [ordered]@{
        enum = 'CREATE TYPE.+workload_state.+ENUM'
        default = 'DEFAULT ''Pending'''
        json = 'jsonb'
        foreignKey = 'FOREIGN KEY'
        checkConstraint = 'CHECK \(replicas >= 0\)'
        partialIndex = 'WHERE \(deleted_at IS NULL\)'
    }
    foreach ($entry in $requiredPatterns.GetEnumerator()) {
        if ($firstSql -notmatch $entry.Value) { throw "Generated migration omitted $($entry.Key)." }
    }

    Invoke-Atlas @('migrate', 'apply', '--dir', 'file:///migrations', '--url', $targetUrl)

    Invoke-Atlas @(
        'migrate', 'diff', '0002_rename_and_drop',
        '--dir', 'file:///migrations',
        '--to', 'file:///work/rust/schema-tooling/proof/schema-v2.sql',
        '--dev-url', $devUrl
    )
    $secondMigration = @(Get-ChildItem $temporaryDirectory -File -Filter '*_0002_rename_and_drop.sql')[0]
    if (-not $secondMigration) { throw 'Atlas did not generate the rename proof migration.' }
    $secondSql = Get-Content $secondMigration.FullName -Raw
    if ($secondSql -notmatch 'RENAME COLUMN.+external_id.+runtime_id') {
        throw "The declarative rename intent did not generate a PostgreSQL column rename.`n$secondSql"
    }
    if ($secondSql -notmatch 'DROP COLUMN.+legacy_note') {
        throw 'The destructive-change fixture did not produce an observable drop.'
    }

    $sourceSql = Join-Path $repoRoot 'src\Citadel.Infrastructure\Scripts\script0001.sql'
    Get-Content $sourceSql -Raw | docker exec -i $databaseContainer psql -v ON_ERROR_STOP=1 -U postgres -d current *> $null
    if ($LASTEXITCODE -ne 0) { throw 'The accepted .NET development baseline did not apply to PostgreSQL 18.1.' }

    Invoke-Atlas @('schema', 'inspect', '--url', $currentUrl, '--format', '{{ json . }}') | Out-Null
    $tableCount = docker exec $databaseContainer psql -U postgres -d current -Atc "SELECT count(*) FROM pg_tables WHERE schemaname = 'public';"
    if ($LASTEXITCODE -ne 0 -or [int]$tableCount -ne 83) {
        throw "Expected 83 accepted baseline tables, observed '$tableCount'."
    }

    Write-Host 'Phase 1 schema tooling proof passed: defaults, foreign keys, unique/partial indexes, checks, enum/JSON types, declarative rename intent, destructive output, and the 83-table accepted baseline.'
}
finally {
    Remove-ProofResources
    if (Test-Path $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}
