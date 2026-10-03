$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'BuildCache.ps1')

# Mock Docker: these tests never modify real volumes or require a daemon.
$script:calls = [Collections.Generic.List[string]]::new()
function docker {
    $script:calls.Add(($args -join ' '))
    $global:LASTEXITCODE = 0
    if ($args[1] -eq 'ls') {
        if ($script:unavailable) { $global:LASTEXITCODE = 1 }
        elseif ($script:exists) { $script:name }
    }
    elseif ($args[1] -eq 'rm' -and $script:inUse) { $global:LASTEXITCODE = 1 }
}
function Assert([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$script:name = New-CitadelBuildCacheName
Assert ($script:name -cmatch '^citadel-rust-test-[a-f0-9]{32}$') 'Unexpected cache name'
Assert ($script:name -ne (New-CitadelBuildCacheName)) 'Concurrent suites must use distinct volumes'
$script:exists = $true
$script:unavailable = $false
$script:inUse = $false
Remove-CitadelBuildCache $script:name
Assert ($script:calls.Count -eq 2) 'Expected lookup and exact removal'
Assert ($script:calls[1] -ceq "volume rm $script:name") 'Never prune or force-delete volumes'

foreach ($unsafe in @('postgres-data', 'citadel-rust-target', 'citadel-rust-test-', '../target')) {
    $script:calls.Clear()
    $rejected = $false
    try { Remove-CitadelBuildCache $unsafe } catch { $rejected = $true }
    Assert $rejected 'Unowned volume must be rejected'
    Assert ($script:calls.Count -eq 0) 'Unsafe input must not call Docker'
}

$script:exists = $false
$script:calls.Clear()
Remove-CitadelBuildCache $script:name
Assert ($script:calls.Count -eq 1) 'Absent cache must not be removed'
$script:unavailable = $true
$script:calls.Clear()
$warnings = @(Remove-CitadelBuildCache $script:name 3>&1)
Assert ($script:calls.Count -eq 1 -and $warnings.Count -eq 1) 'Daemon failure must warn without removal'
$script:unavailable = $false
$script:exists = $true
$script:inUse = $true
$script:calls.Clear()
$warnings = @(Remove-CitadelBuildCache $script:name 3>&1)
Assert ($script:calls.Count -eq 2 -and $warnings.Count -eq 1) 'In-use cache must not trigger a forced retry'

foreach ($file in Get-ChildItem -LiteralPath @($PSScriptRoot, (Join-Path $PSScriptRoot '../performance')) -Filter '*.ps1') {
    $parseErrors = $null
    $tokens = $null
    $ast = [Management.Automation.Language.Parser]::ParseFile($file.FullName, [ref]$tokens, [ref]$parseErrors)
    Assert ($parseErrors.Count -eq 0) "PowerShell syntax error in $($file.Name): $parseErrors"
    $content = Get-Content -LiteralPath $file.FullName -Raw
    if ($file.FullName -ne $PSCommandPath -and $file.Name -like 'Test-*' -and $content.Contains('New-CitadelBuildCacheName')) {
        Assert (-not $content.Contains("'citadel-rust-target:")) "Shared target remains in $($file.Name)"
        $finalizers = $ast.FindAll({ param($node)
            $node -is [Management.Automation.Language.TryStatementAst] -and
            $node.Finally -and $node.Finally.Extent.Text.Contains('Remove-CitadelBuildCache')
        }, $true)
        Assert ($finalizers.Count -eq 1) "Missing cleanup finalizer in $($file.Name)"
    }
}
Write-Output 'Build-cache safety and script syntax tests passed.'
