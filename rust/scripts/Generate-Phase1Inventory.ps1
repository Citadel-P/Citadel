param(
    [switch]$Check
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$outputDirectory = Join-Path $repoRoot 'rust\phase1'
$inventoryPath = Join-Path $outputDirectory 'inventory.json'
$contractPath = Join-Path $outputDirectory 'accepted-contracts.json'
$configurationPath = Join-Path $outputDirectory 'configuration-defaults.json'

function Get-RelativePath([string]$Path) {
    $fullPath = [IO.Path]::GetFullPath($Path)
    $prefix = $repoRoot.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if (-not $fullPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path '$fullPath' is outside the repository root."
    }
    return $fullPath.Substring($prefix.Length).Replace('\', '/')
}

function Get-SourceFiles([string]$Directory, [string]$Filter) {
    return @(Get-ChildItem (Join-Path $repoRoot $Directory) -Recurse -File -Filter $Filter |
        Where-Object { $_.FullName -notmatch '[\\/](?:bin|obj|node_modules|target)[\\/]' } |
        Sort-Object FullName)
}

function Get-LineMatches([IO.FileInfo[]]$Files, [string]$Pattern, [string]$NameGroup = '') {
    $result = [Collections.Generic.List[object]]::new()
    foreach ($file in $Files) {
        $lineNumber = 0
        foreach ($line in [IO.File]::ReadLines($file.FullName)) {
            $lineNumber++
            $match = [regex]::Match($line, $Pattern)
            if (-not $match.Success) { continue }
            $name = if ($NameGroup) { $match.Groups[$NameGroup].Value } else { $line.Trim() }
            $result.Add([ordered]@{
                name = $name
                file = Get-RelativePath $file.FullName
                line = $lineNumber
            })
        }
    }
    return @($result)
}

function Get-OpenApiOperations([string]$RelativePath) {
    $document = Get-Content (Join-Path $repoRoot $RelativePath) -Raw | ConvertFrom-Json
    $operations = [Collections.Generic.List[object]]::new()
    $methods = @('get', 'post', 'put', 'patch', 'delete', 'options', 'head', 'trace')
    foreach ($pathProperty in @($document.paths.PSObject.Properties | Sort-Object Name)) {
        foreach ($method in $methods) {
            $operation = $pathProperty.Value.$method
            if ($null -eq $operation -or [string]::IsNullOrWhiteSpace($operation.operationId)) { continue }
            $operations.Add([ordered]@{
                operationId = [string]$operation.operationId
                method = $method.ToUpperInvariant()
                path = $pathProperty.Name
                tags = @($operation.tags)
            })
        }
    }
    return @($operations | Sort-Object operationId)
}

function Get-FrontendOperations {
    $relativePath = 'src/Citadel.FrontEnd/src/api/generated/resources.ts'
    $path = Join-Path $repoRoot $relativePath
    $operations = [Collections.Generic.List[object]]::new()
    $lineNumber = 0
    $pattern = '^\s*(?<name>[A-Za-z0-9_]+): \{ method: "(?<method>[A-Z]+)", key: "[^"]+", path: "(?<path>[^"]+)", tag: "(?<tag>[^"]+)"'
    foreach ($line in [IO.File]::ReadLines($path)) {
        $lineNumber++
        $match = [regex]::Match($line, $pattern)
        if (-not $match.Success) { continue }
        $operations.Add([ordered]@{
            operationId = $match.Groups['name'].Value
            method = $match.Groups['method'].Value
            path = $match.Groups['path'].Value
            tag = $match.Groups['tag'].Value
            file = $relativePath
            line = $lineNumber
        })
    }
    return @($operations | Sort-Object operationId)
}

function Get-RepositorySurface {
    $relativePath = 'src/Citadel.Domain/Contracts/Interfaces/IUnitOfWork.cs'
    $path = Join-Path $repoRoot $relativePath
    $repositories = [Collections.Generic.List[object]]::new()
    $methods = [Collections.Generic.List[object]]::new()
    $currentInterface = $null
    $declaration = ''
    $declarationLine = 0
    $lineNumber = 0

    foreach ($line in [IO.File]::ReadLines($path)) {
        $lineNumber++
        $interfaceMatch = [regex]::Match($line, '^public interface (?<name>I[A-Za-z0-9_]+)')
        if ($interfaceMatch.Success) {
            $currentInterface = $interfaceMatch.Groups['name'].Value
            $declaration = ''
        }

        $repositoryMatch = [regex]::Match($line, '^\s*(?<type>I[A-Za-z0-9_]+Repository)\s+(?<name>[A-Za-z0-9_]+)\s+\{ get; \}')
        if ($repositoryMatch.Success -and $currentInterface -eq 'IUnitOfWork') {
            $repositories.Add([ordered]@{
                name = $repositoryMatch.Groups['name'].Value
                interface = $repositoryMatch.Groups['type'].Value
                file = $relativePath
                line = $lineNumber
            })
        }

        if (-not $currentInterface) { continue }
        if (-not $declaration -and $line -match '^\s*(Task(?:<|\s)|IAsyncEnumerable<)') {
            $declaration = $line.Trim()
            $declarationLine = $lineNumber
        } elseif ($declaration) {
            $declaration += ' ' + $line.Trim()
        }

        if ($declaration -and $declaration.Contains(';')) {
            $methodMatch = [regex]::Match($declaration, '\b(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(')
            if ($methodMatch.Success) {
                $methods.Add([ordered]@{
                    interface = $currentInterface
                    name = $methodMatch.Groups['name'].Value
                    signature = ($declaration -replace '\s+', ' ').Trim()
                    file = $relativePath
                    line = $declarationLine
                })
            }
            $declaration = ''
        }
    }

    return [ordered]@{
        repositories = @($repositories | Sort-Object name)
        methods = @($methods | Sort-Object interface, name, line)
    }
}

function Get-ProtobufSurface {
    $directory = Join-Path $repoRoot 'src\Citadel.Contracts\src\Citadel.Hosting.DockerClient\Protos'
    $files = @(Get-ChildItem $directory -File -Filter '*.proto' | Sort-Object Name)
    $services = [Collections.Generic.List[object]]::new()
    $messages = [Collections.Generic.List[object]]::new()
    $currentService = $null
    foreach ($file in $files) {
        $lineNumber = 0
        foreach ($line in [IO.File]::ReadLines($file.FullName)) {
            $lineNumber++
            $service = [regex]::Match($line, '^service\s+(?<name>[A-Za-z0-9_]+)')
            if ($service.Success) {
                $currentService = $service.Groups['name'].Value
                continue
            }
            $message = [regex]::Match($line, '^(?:message|enum)\s+(?<name>[A-Za-z0-9_]+)')
            if ($message.Success) {
                $messages.Add([ordered]@{
                    name = $message.Groups['name'].Value
                    file = Get-RelativePath $file.FullName
                    line = $lineNumber
                })
                continue
            }
            $rpc = [regex]::Match($line, '^\s*rpc\s+(?<name>[A-Za-z0-9_]+)\s*\((?<request>[^)]+)\)\s*returns\s*\((?<response>[^)]+)\)')
            if ($rpc.Success) {
                $services.Add([ordered]@{
                    service = $currentService
                    name = $rpc.Groups['name'].Value
                    request = $rpc.Groups['request'].Value.Trim()
                    response = $rpc.Groups['response'].Value.Trim()
                    file = Get-RelativePath $file.FullName
                    line = $lineNumber
                })
            }
        }
    }
    return [ordered]@{
        files = @($files | ForEach-Object { Get-RelativePath $_.FullName })
        messages = @($messages | Sort-Object file, line)
        rpcs = @($services | Sort-Object file, line)
    }
}

function Get-Tables {
    $relativePath = 'src/Citadel.Infrastructure/Scripts/script0001.sql'
    $text = Get-Content (Join-Path $repoRoot $relativePath) -Raw
    $matches = [regex]::Matches($text, '(?im)^CREATE TABLE (?:IF NOT EXISTS )?(?:"[^"]+"\.)?"?(?<name>[^"\s(]+)"?\s*\(')
    return @($matches | ForEach-Object { $_.Groups['name'].Value } | Sort-Object -Unique)
}

function Get-FileContract([string]$RelativePath, [string]$Kind) {
    $path = Join-Path $repoRoot $RelativePath
    $item = Get-Item $path
    return [ordered]@{
        kind = $Kind
        path = $RelativePath.Replace('\', '/')
        sha256 = (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant()
        bytes = $item.Length
    }
}

function ConvertTo-RedactedValue([string]$Name, [object]$Value) {
    if ($Name -match '(?i)(password|secret|credential|encryptionkey|jwt(?:::|__)key$|(?:^|__|:)token$|api_?key)') { return '<redacted-or-generated>' }
    return $Value
}

function Add-JsonDefaults([Collections.Generic.List[object]]$Target, [object]$Value, [string]$Prefix, [string]$Source) {
    foreach ($property in @($Value.PSObject.Properties | Sort-Object Name)) {
        $name = if ($Prefix) { "$Prefix`:$($property.Name)" } else { $property.Name }
        if ($property.Value -is [Management.Automation.PSCustomObject]) {
            Add-JsonDefaults $Target $property.Value $name $Source
        } elseif ($property.Value -is [Array]) {
            $index = 0
            foreach ($entry in $property.Value) {
                $Target.Add([ordered]@{ name = "$name`:$index"; value = ConvertTo-RedactedValue $name $entry; source = $Source })
                $index++
            }
        } else {
            $Target.Add([ordered]@{ name = $name; value = ConvertTo-RedactedValue $name $property.Value; source = $Source })
        }
    }
}

$csharpFiles = Get-SourceFiles 'src' '*.cs'
$applicationModule = Get-Item (Join-Path $repoRoot 'src\Citadel.Application\ApplicationModule.cs')
$dockerClientFiles = Get-SourceFiles 'src\Citadel.Contracts\src\Citadel.Hosting.DockerClient\HttpClient' '*.cs'
$repositorySurface = Get-RepositorySurface
$protobufSurface = Get-ProtobufSurface
$fullOperations = Get-OpenApiOperations 'src/schema/Citadel.WebApi.json'
$publicOperations = Get-OpenApiOperations 'src/schema/Citadel.WebApi_public.json'
$frontendOperations = Get-FrontendOperations

$hostedServices = Get-LineMatches @($applicationModule) 'AddHostedService(?:<|\(provider => provider\.GetRequiredService<|\(s => s\.GetRequiredService<)(?<name>[A-Za-z0-9_]+)' 'name'
$boundedQueues = Get-LineMatches $csharpFiles 'Channel\.CreateBounded'
$serializationRoots = Get-LineMatches $csharpFiles '\[JsonSerializable\(typeof\((?<name>.+)\)\)\]' 'name'
$realtimeSurface = Get-LineMatches $csharpFiles '(?<name>AddToGroupAsync|RemoveFromGroupAsync|Clients\.Group|SendAsync|SendCoreAsync|StreamManager|GroupName)' 'name'
$externalProcesses = Get-LineMatches $csharpFiles '(?<name>new ProcessStartInfo|Process\.Start|new Process\()' 'name'
$dockerOperations = Get-LineMatches $dockerClientFiles '^\s*\[(?:Get|Post|Put|Patch|Delete)\("(?<name>[^"]+)' 'name'
$connectorFiles = @($csharpFiles | Where-Object { $_.Name -match 'Connector|IConnector' -or $_.DirectoryName -match 'Connectors' })
$connectorOperations = Get-LineMatches $connectorFiles '^\s*(?:public\s+)?(?:async\s+)?(?:Task|IAsyncEnumerable|ValueTask)(?:<[^;]+?>)?\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(' 'name'

$inventory = [ordered]@{
    schemaVersion = 1
    source = 'Citadel .NET implementation and generated contracts'
    summary = [ordered]@{
        apiOperations = $fullOperations.Count
        publicApiOperations = $publicOperations.Count
        activeFrontendOperations = $frontendOperations.Count
        unitOfWorkRepositories = $repositorySurface.repositories.Count
        repositoryMethods = $repositorySurface.methods.Count
        hostedServices = $hostedServices.Count
        boundedQueues = $boundedQueues.Count
        serializationRoots = $serializationRoots.Count
        realtimeReferences = $realtimeSurface.Count
        connectorOperations = $connectorOperations.Count
        protobufFiles = $protobufSurface.files.Count
        protobufMessages = $protobufSurface.messages.Count
        protobufRpcs = $protobufSurface.rpcs.Count
        databaseTables = (Get-Tables).Count
        dockerHttpOperations = $dockerOperations.Count
        externalProcessSites = $externalProcesses.Count
    }
    apiOperations = $fullOperations
    publicApiOperations = $publicOperations
    activeFrontendOperations = $frontendOperations
    unitOfWorkRepositories = $repositorySurface.repositories
    repositoryMethods = $repositorySurface.methods
    hostedServices = $hostedServices
    boundedQueues = $boundedQueues
    serializationRoots = $serializationRoots
    realtimeReferences = $realtimeSurface
    connectorOperations = $connectorOperations
    protobuf = $protobufSurface
    databaseTables = Get-Tables
    dockerHttpOperations = $dockerOperations
    externalProcessSites = $externalProcesses
}

$contracts = [Collections.Generic.List[object]]::new()
$contracts.Add((Get-FileContract 'src/schema/Citadel.WebApi.json' 'openapi-full'))
$contracts.Add((Get-FileContract 'src/schema/Citadel.WebApi_public.json' 'openapi-public'))
$contracts.Add((Get-FileContract 'src/Citadel.FrontEnd/src/api/generated/resources.ts' 'frontend-operation-catalog'))
$contracts.Add((Get-FileContract 'src/Citadel.FrontEnd/src/api/generated/api.types.ts' 'frontend-types'))
$contracts.Add((Get-FileContract 'src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/HttpClient/v1.49.yaml' 'docker-openapi'))
$contracts.Add((Get-FileContract 'src/Citadel.Infrastructure/Scripts/script0001.sql' 'dotnet-development-baseline-evidence'))
foreach ($proto in $protobufSurface.files) { $contracts.Add((Get-FileContract $proto 'protobuf-source')) }
$contractManifest = [ordered]@{
    schemaVersion = 1
    note = 'Hashes freeze accepted Phase 1 inputs; sources remain the canonical language-neutral descriptors.'
    files = @($contracts | Sort-Object kind, path)
}

$defaults = [Collections.Generic.List[object]]::new()
$appSettingsPath = 'src/Citadel.WebApi/appsettings.json'
$appSettings = Get-Content (Join-Path $repoRoot $appSettingsPath) -Raw | ConvertFrom-Json
Add-JsonDefaults $defaults $appSettings '' $appSettingsPath
$envExamplePath = Join-Path $repoRoot '.env.example'
$lineNumber = 0
foreach ($line in [IO.File]::ReadLines($envExamplePath)) {
    $lineNumber++
    $match = [regex]::Match($line, '^\s*#?\s*(?<name>[A-Za-z][A-Za-z0-9_]*)=(?<value>.*)$')
    if (-not $match.Success) { continue }
    $name = $match.Groups['name'].Value
    $value = ConvertTo-RedactedValue $name $match.Groups['value'].Value
    $defaults.Add([ordered]@{ name = $name; value = $value; source = ".env.example:$lineNumber" })
}
$configuration = [ordered]@{
    schemaVersion = 1
    note = 'Local .env files are excluded. Sensitive defaults and placeholders are redacted.'
    values = @($defaults | Sort-Object name, source)
}

New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$outputs = [ordered]@{
    $inventoryPath = ($inventory | ConvertTo-Json -Depth 12) + "`n"
    $contractPath = ($contractManifest | ConvertTo-Json -Depth 8) + "`n"
    $configurationPath = ($configuration | ConvertTo-Json -Depth 8) + "`n"
}

$stale = [Collections.Generic.List[string]]::new()
foreach ($entry in $outputs.GetEnumerator()) {
    $existing = if (Test-Path $entry.Key) { Get-Content $entry.Key -Raw } else { $null }
    if ($existing -eq $entry.Value) { continue }
    if ($Check) {
        $stale.Add((Get-RelativePath $entry.Key))
    } else {
        [IO.File]::WriteAllText($entry.Key, $entry.Value, [Text.UTF8Encoding]::new($false))
    }
}

if ($stale.Count -gt 0) {
    throw "Phase 1 inventory is stale: $($stale -join ', '). Run rust/scripts/Generate-Phase1Inventory.ps1."
}

Write-Host "Phase 1 inventory verified: $($fullOperations.Count) API operations, $($frontendOperations.Count) frontend operations, $($protobufSurface.rpcs.Count) protobuf RPCs."
