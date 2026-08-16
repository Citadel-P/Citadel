$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$phaseDirectory = Join-Path $repoRoot 'rust\phase1'

& (Join-Path $PSScriptRoot 'Generate-Phase1Inventory.ps1') -Check

$inventory = Get-Content (Join-Path $phaseDirectory 'inventory.json') -Raw | ConvertFrom-Json
$contracts = Get-Content (Join-Path $phaseDirectory 'accepted-contracts.json') -Raw | ConvertFrom-Json
$configuration = Get-Content (Join-Path $phaseDirectory 'configuration-defaults.json') -Raw | ConvertFrom-Json
$traceability = Get-Content (Join-Path $phaseDirectory 'traceability.json') -Raw | ConvertFrom-Json
$breakingChanges = Get-Content (Join-Path $phaseDirectory 'breaking-changes.json') -Raw | ConvertFrom-Json

function Assert([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Assert-Unique([object[]]$Items, [scriptblock]$Selector, [string]$Description) {
    $values = @($Items | ForEach-Object $Selector)
    Assert (($values | Sort-Object -Unique).Count -eq $values.Count) "$Description contains duplicates."
}

function Normalize-PathTemplate([string]$Path) {
    return [regex]::Replace($Path, '\{[^}]+\}', '{}')
}

Assert ($inventory.schemaVersion -eq 1) 'Unsupported inventory schema version.'
Assert ($contracts.schemaVersion -eq 1) 'Unsupported contract manifest schema version.'
Assert ($configuration.schemaVersion -eq 1) 'Unsupported configuration inventory schema version.'
Assert ($traceability.schemaVersion -eq 1) 'Unsupported traceability schema version.'
Assert ($breakingChanges.schemaVersion -eq 1) 'Unsupported breaking-change schema version.'

$fullOperations = @($inventory.apiOperations)
$frontendOperations = @($inventory.activeFrontendOperations)
$publicOperations = @($inventory.publicApiOperations)
Assert-Unique $fullOperations { $_.operationId } 'Full OpenAPI operation IDs'
Assert-Unique $frontendOperations { $_.operationId } 'Frontend operation IDs'
Assert-Unique $publicOperations { $_.operationId } 'Public OpenAPI operation IDs'

$fullById = @{}
foreach ($operation in $fullOperations) { $fullById[$operation.operationId] = $operation }
foreach ($operation in $frontendOperations) {
    Assert ($fullById.ContainsKey($operation.operationId)) "Frontend operation '$($operation.operationId)' is absent from OpenAPI."
    $source = $fullById[$operation.operationId]
    Assert ($source.method -eq $operation.method) "Method mismatch for '$($operation.operationId)'."
    Assert ($source.path -eq $operation.path) "Path mismatch for '$($operation.operationId)'."
}
Assert ($frontendOperations.Count -eq $fullOperations.Count) 'The active frontend does not cover every full OpenAPI operation.'
foreach ($operation in $publicOperations) {
    Assert ($fullById.ContainsKey($operation.operationId)) "Public operation '$($operation.operationId)' is absent from full OpenAPI."
    $source = $fullById[$operation.operationId]
    Assert (
        $source.method -eq $operation.method -and
        (Normalize-PathTemplate $source.path) -eq (Normalize-PathTemplate $operation.path)
    ) "Public operation '$($operation.operationId)' differs from full OpenAPI."
}

Assert ($inventory.summary.protobufFiles -eq 10) 'The accepted Agent/Edge contract must contain all ten protobuf files.'
Assert ($inventory.summary.protobufRpcs -eq 69) 'The accepted protobuf RPC characterization changed; review and update Phase 1 deliberately.'
Assert ($inventory.summary.databaseTables -eq 83) 'The accepted .NET baseline table characterization changed; review and update Phase 1 deliberately.'
Assert (@($inventory.databaseTables) -contains '__EFMigrationsHistory') 'The .NET evidence inventory no longer identifies EF history separately.'
Assert ($inventory.summary.boundedQueues -eq 27) 'The bounded-queue characterization changed; review ownership before updating it.'
Assert ($inventory.summary.unitOfWorkRepositories -eq 56) 'The repository-capability characterization changed; review ownership before updating it.'

Assert-Unique @($contracts.files) { $_.path } 'Contract manifest paths'
Assert (@($contracts.files | Where-Object kind -eq 'protobuf-source').Count -eq 10) 'The contract manifest must pin all protobuf sources.'
foreach ($contract in $contracts.files) {
    $path = Join-Path $repoRoot $contract.path
    Assert (Test-Path $path -PathType Leaf) "Pinned contract '$($contract.path)' does not exist."
    $actualHash = (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant()
    Assert ($actualHash -eq $contract.sha256) "Pinned contract '$($contract.path)' has drifted."
    Assert ((Get-Item $path).Length -eq $contract.bytes) "Pinned contract '$($contract.path)' size has drifted."
}

Assert (@($configuration.values).Count -gt 0) 'No repository configuration defaults were captured.'
foreach ($value in $configuration.values) {
    Assert (-not $value.source.StartsWith('.env:', [StringComparison]::OrdinalIgnoreCase)) 'A local .env value entered the configuration inventory.'
    if ($value.name -match '(?i)(password|secret|credential|encryptionkey|jwt(?:::|__)key$|(?:^|__|:)token$|api_?key)') {
        Assert ($value.value -eq '<redacted-or-generated>') "Sensitive configuration '$($value.name)' was not redacted."
    }
}

$expectedSpecifications = @(
    'automation-actions.md', 'backup-and-restore.md', 'docker-builds.md',
    'docker-swarm-v1.md', 'duplicate-deployment-stack.md', 'edge-agent.md',
    'external-build-agents.md', 'first-run-administrator-bootstrap.md',
    'frontend-and-product-acceptance-testing.md', 'fumadocs-release-documentation.md',
    'git-repository-browser.md', 'git-repository-gitops.md', 'git-stack-integration.md',
    'global-search.md', 'live-updates-connection-status.md', 'manual-stack-auto-update.md',
    'offline-licensing.md', 'oidc-provider-integration.md', 'on-demand-update-checks.md',
    'platform-disk-usage-monitoring.md', 'project-guidelines.md', 'release-procedure.md',
    'resource-tags.md', 'service-accounts.md', 'stack.md', 'stack-drift.md',
    'swarm-managed-services.md', 'swarm-node-agent-data-plane.md', 'system-containers.md',
    'tls-termination-and-agent-transport.md', 'two-factor-authentication.md',
    'unmanaged-resource-adoption.md', 'user-profile.md', 'variables-and-secrets.md',
    'volume-browser-and-stack-backup.md', 'webhook-listener.md'
)
$actualSpecifications = @($traceability.entries.spec | Sort-Object -Unique)
Assert ($actualSpecifications.Count -eq $expectedSpecifications.Count) 'Traceability does not cover every required specification exactly once.'
foreach ($specification in $expectedSpecifications) {
    Assert ($actualSpecifications -contains $specification) "Traceability is missing '$specification'."
}
Assert-Unique @($traceability.entries) { $_.id } 'Traceability IDs'
foreach ($entry in $traceability.entries) {
    foreach ($property in @('sections', 'classification', 'requirement', 'dotnetOwner', 'testEvidence', 'rustOwner', 'rustPhase', 'disposition')) {
        Assert (-not [string]::IsNullOrWhiteSpace($entry.$property)) "Traceability '$($entry.id)' has no $property."
    }
    foreach ($property in @('dotnetOwner', 'testEvidence')) {
        foreach ($evidencePath in ($entry.$property -split '; ')) {
            $resolvedEvidencePath = if ($evidencePath -eq 'Citadel.Agent') {
                Join-Path (Split-Path $repoRoot -Parent) 'Citadel.Agent'
            } else {
                Join-Path $repoRoot $evidencePath
            }
            Assert (Test-Path $resolvedEvidencePath) "Traceability '$($entry.id)' references missing evidence '$evidencePath'."
        }
    }
}

Assert (@($breakingChanges.entries).Count -gt 0) 'The closed breaking-change ledger is empty.'
Assert-Unique @($breakingChanges.entries) { $_.id } 'Breaking-change IDs'
foreach ($entry in $breakingChanges.entries) {
    foreach ($property in @('feature', 'oldBehavior', 'newBehavior', 'reason', 'impact', 'proof', 'status')) {
        Assert (-not [string]::IsNullOrWhiteSpace($entry.$property)) "Breaking change '$($entry.id)' has no $property."
    }
}

Write-Host "Phase 1 contract tests passed: $($fullOperations.Count) HTTP operations, $($inventory.summary.protobufRpcs) protobuf RPCs, $($actualSpecifications.Count) specification entries."
