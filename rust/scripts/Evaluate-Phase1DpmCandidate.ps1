param(
    [switch]$SkipImageBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$toolImage = 'citadel-dpm:0.3.2'
$postgresImage = 'postgres@sha256:aa6eb304ddb6dd26df23d05db4e5cb05af8951cda3e0dc57731b771e0ef4ab29'
$network = 'citadel-rust-phase1-dpm'
$databaseContainer = 'citadel-rust-phase1-dpm-db'
$temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$temporaryDirectory = Join-Path $temporaryRoot ('citadel-phase1-dpm-' + [Guid]::NewGuid().ToString('N'))
$password = 'phase1-dpm-proof'

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

function Invoke-Dpm([string[]]$Arguments) {
    $mountProof = "type=bind,source=$temporaryDirectory,target=/proof"
    $dockerArguments = @(
        'run', '--rm', '--network', $network,
        '--mount', $mountProof,
        $toolImage
    ) + $Arguments
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & docker @dockerArguments 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($exitCode -ne 0) {
        $output | Write-Output
        throw 'DPM schema command failed.'
    }
    $output | Where-Object { $_ -match '^dpm:' } | Write-Host
}

try {
    New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null

    if (-not $SkipImageBuild) {
        Invoke-Docker @(
            'build', '--file', 'rust/schema-tooling/Dockerfile.dpm',
            '--tag', $toolImage, '.'
        ) 'Building the pinned DPM candidate failed.'
    }

    $sourcePath = Join-Path $repoRoot 'src\Citadel.Infrastructure\Scripts\script0001.sql'
    $acceptedSchema = [IO.File]::ReadAllText($sourcePath)
    $historyTablePattern = '(?s)\ACREATE TABLE IF NOT EXISTS "__EFMigrationsHistory" \(.*?\);\r?\n\r?\n'
    $historyInsertPattern = '(?s)\r?\nINSERT INTO "__EFMigrationsHistory" \("MigrationId", "ProductVersion"\)\r?\nVALUES \(.*?\);\r?\n'

    if ([regex]::Matches($acceptedSchema, $historyTablePattern).Count -ne 1) {
        throw 'The generated baseline no longer has the expected EF history table block.'
    }
    if ([regex]::Matches($acceptedSchema, $historyInsertPattern).Count -ne 1) {
        throw 'The generated baseline no longer has the expected EF history insert block.'
    }

    $productSchema = [regex]::Replace($acceptedSchema, $historyTablePattern, '')
    $productSchema = [regex]::Replace($productSchema, $historyInsertPattern, "`r`n")
    if ($productSchema.IndexOf('__EFMigrationsHistory', [StringComparison]::Ordinal) -ge 0) {
        throw 'EF migration metadata leaked into the product schema proof.'
    }
    if ([regex]::Matches($productSchema, '(?m)^CREATE TABLE ').Count -ne 82) {
        throw 'The product schema proof must contain exactly 82 Citadel tables.'
    }
    if ([regex]::Matches($productSchema, '(?m)^INSERT INTO ').Count -ne 92) {
        throw 'The product schema proof did not retain every generated seed insert.'
    }

    $productSchemaPath = Join-Path $temporaryDirectory 'product-schema.sql'
    [IO.File]::WriteAllText($productSchemaPath, $productSchema, [Text.UTF8Encoding]::new($false))

    Remove-ProofResources
    Invoke-Docker @('network', 'create', $network) 'Creating the DPM proof network failed.'
    Invoke-Docker @(
        'run', '--detach', '--name', $databaseContainer, '--network', $network,
        '--env', "POSTGRES_PASSWORD=$password",
        '--env', 'POSTGRES_DB=accepted',
        $postgresImage
    ) 'Starting the DPM proof database failed.'

    $ready = $false
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            & docker exec $databaseContainer psql -U postgres -d accepted -Atc 'SELECT 1;' *> $null
            if ($LASTEXITCODE -eq 0) { $ready = $true; break }
            Start-Sleep -Milliseconds 500
        }
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if (-not $ready) { throw 'The DPM proof database did not become ready.' }

    Invoke-Docker @('exec', $databaseContainer, 'createdb', '-U', 'postgres', 'target') 'Creating the DPM target database failed.'
    Invoke-Docker @('exec', $databaseContainer, 'createdb', '-U', 'postgres', 'destructive') 'Creating the DPM destructive-change database failed.'
    Get-Content -LiteralPath $productSchemaPath -Raw |
        docker exec -i $databaseContainer psql -v ON_ERROR_STOP=1 -U postgres -d accepted *> $null
    if ($LASTEXITCODE -ne 0) { throw 'The generated Citadel product baseline did not apply.' }

    $schemaV1Path = Join-Path $repoRoot 'rust\schema-tooling\proof\schema-v1.sql'
    $schemaV2Path = Join-Path $repoRoot 'rust\schema-tooling\proof\schema-v2.sql'
    [IO.File]::WriteAllText(
        (Join-Path $temporaryDirectory 'schema-v2.sql'),
        [IO.File]::ReadAllText($schemaV2Path),
        [Text.UTF8Encoding]::new($false))
    Get-Content -LiteralPath $schemaV1Path -Raw |
        docker exec -i $databaseContainer psql -v ON_ERROR_STOP=1 -U postgres -d destructive *> $null
    if ($LASTEXITCODE -ne 0) { throw 'The destructive-change fixture did not apply.' }

    $acceptedUrl = "postgres://postgres:$password@$databaseContainer`:5432/accepted?sslmode=disable"
    $targetUrl = "postgres://postgres:$password@$databaseContainer`:5432/target?sslmode=disable"
    $destructiveUrl = "postgres://postgres:$password@$databaseContainer`:5432/destructive?sslmode=disable"
    $shadowUrl = "postgres://postgres:$password@$databaseContainer`:5432/postgres?sslmode=disable"

    Invoke-Dpm @(
        'diff', '--source-sql', '/proof/schema-v2.sql',
        '--target', $destructiveUrl, '--shadow', $shadowUrl,
        '--out', '/proof/gated-destructive.sql'
    )
    $gatedSql = [IO.File]::ReadAllText((Join-Path $temporaryDirectory 'gated-destructive.sql'))
    if (
        $gatedSql -notmatch '(?m)^-- ALTER TABLE .* DROP COLUMN IF EXISTS "legacy_note";' -or
        $gatedSql -notmatch '(?m)^-- changes: \d+ total, [1-9]\d* destructive \([1-9]\d* gated\),'
    ) {
        throw 'DPM did not visibly gate the destructive column removal.'
    }

    Invoke-Dpm @(
        'diff', '--source-sql', '/proof/schema-v2.sql',
        '--target', $destructiveUrl, '--shadow', $shadowUrl,
        '--allow-destructive-sql', '--out', '/proof/reviewable-destructive.sql'
    )
    $reviewableSql = [IO.File]::ReadAllText((Join-Path $temporaryDirectory 'reviewable-destructive.sql'))
    if ($reviewableSql -notmatch '(?m)^ALTER TABLE .* DROP COLUMN IF EXISTS "legacy_note";') {
        throw 'DPM did not emit reviewable destructive SQL after explicit generation consent.'
    }

    Invoke-Dpm @(
        'verify', '--source-sql', '/proof/product-schema.sql',
        '--target', $targetUrl, '--shadow', $shadowUrl,
        '--out', '/proof/verified-migration.sql'
    )
    Invoke-Dpm @(
        'apply', '--source-sql', '/proof/product-schema.sql',
        '--target', $targetUrl, '--shadow', $shadowUrl, '--yes',
        '--out', '/proof/applied-migration.sql'
    )
    Invoke-Dpm @(
        'diff', '--source', $acceptedUrl, '--target', $targetUrl,
        '--fail-on-diff', '--out', '/proof/convergence.sql'
    )

    $catalogMetrics = @'
SELECT json_build_object(
    'tables', (SELECT count(*) FROM pg_tables WHERE schemaname = 'public'),
    'foreign_keys', (SELECT count(*) FROM pg_constraint WHERE contype = 'f' AND connamespace = 'public'::regnamespace),
    'checks', (SELECT count(*) FROM pg_constraint WHERE contype = 'c' AND connamespace = 'public'::regnamespace),
    'unique_indexes', (SELECT count(*) FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'public' AND i.indisunique),
    'partial_indexes', (SELECT count(*) FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'public' AND i.indpred IS NOT NULL),
    'jsonb_columns', (SELECT count(*) FROM information_schema.columns WHERE table_schema = 'public' AND udt_name = 'jsonb'),
    'defaults', (SELECT count(*) FROM information_schema.columns WHERE table_schema = 'public' AND column_default IS NOT NULL),
    'identities', (SELECT count(*) FROM information_schema.columns WHERE table_schema = 'public' AND is_identity = 'YES')
)::text;
'@
    $acceptedMetrics = docker exec $databaseContainer psql -U postgres -d accepted -Atc $catalogMetrics
    if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the accepted schema metrics.' }
    $targetMetrics = docker exec $databaseContainer psql -U postgres -d target -Atc $catalogMetrics
    if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the generated schema metrics.' }
    if ($acceptedMetrics -ne $targetMetrics) {
        throw "DPM did not preserve the accepted schema surface.`nAccepted: $acceptedMetrics`nTarget: $targetMetrics"
    }

    $productTableCount = docker exec $databaseContainer psql -U postgres -d accepted -Atc "SELECT count(*) FROM pg_tables WHERE schemaname = 'public';"
    $roleSeedCount = docker exec $databaseContainer psql -U postgres -d accepted -Atc 'SELECT count(*) FROM roles;'
    if ([int]$productTableCount -ne 82 -or [int]$roleSeedCount -ne 3) {
        throw "The imported baseline has unexpected product/seed counts: tables=$productTableCount roles=$roleSeedCount."
    }

    Write-Host "DPM candidate passed: the generated 82-table product baseline applies with all seeds, and the Rust-native schema diff converges with matching PostgreSQL catalog metrics ($acceptedMetrics)."
}
finally {
    Remove-ProofResources
    $resolvedTemporaryDirectory = [IO.Path]::GetFullPath($temporaryDirectory)
    if (
        $resolvedTemporaryDirectory.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase) -and
        [IO.Path]::GetFileName($resolvedTemporaryDirectory).StartsWith('citadel-phase1-dpm-', [StringComparison]::Ordinal)
    ) {
        if (Test-Path $resolvedTemporaryDirectory) {
            Remove-Item -LiteralPath $resolvedTemporaryDirectory -Recurse -Force
        }
    }
    else {
        throw "Refusing to remove unexpected temporary path '$resolvedTemporaryDirectory'."
    }
}
