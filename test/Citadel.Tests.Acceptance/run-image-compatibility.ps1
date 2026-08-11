[CmdletBinding()]
param(
    [ValidateNotNullOrEmpty()]
    [string] $CoreImage = "citadel-core:acceptance",

    [ValidateNotNullOrEmpty()]
    [string] $AgentImage = "citadel-agent:acceptance",

    [string] $AgentRepository,

    [ValidateSet("Debug", "Release")]
    [string] $Configuration = "Release",

    [string] $ExpectedAgentVersion,

    [switch] $SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$testProject = Join-Path $PSScriptRoot "Citadel.Tests.Acceptance.csproj"

if ([string]::IsNullOrWhiteSpace($AgentRepository)) {
    $AgentRepository = [IO.Path]::GetFullPath(
        (Join-Path $repositoryRoot "..\Citadel.Agent"))
}
elseif (-not [IO.Path]::IsPathRooted($AgentRepository)) {
    $AgentRepository = [IO.Path]::GetFullPath(
        (Join-Path (Get-Location) $AgentRepository))
}

function Invoke-CheckedNative {
    param(
        [Parameter(Mandatory)]
        [string] $FilePath,

        [Parameter(Mandatory)]
        [string[]] $Arguments,

        [Parameter(Mandatory)]
        [string] $Operation
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Operation failed with exit code $LASTEXITCODE."
    }
}

function Assert-DockerImage {
    param(
        [Parameter(Mandatory)]
        [string] $Image
    )

    & docker image inspect $Image *> $null
    if ($LASTEXITCODE -ne 0) {
        throw "Docker image '$Image' is unavailable. Run without -SkipBuild or supply an existing image."
    }
}

if ($SkipBuild) {
    Assert-DockerImage -Image $CoreImage
    Assert-DockerImage -Image $AgentImage
}
else {
    if (-not (Test-Path -LiteralPath $AgentRepository -PathType Container)) {
        throw "Agent repository '$AgentRepository' does not exist. Supply -AgentRepository or use -SkipBuild with an existing Agent image."
    }

    Invoke-CheckedNative `
        -FilePath "docker" `
        -Arguments @(
            "build",
            "--file",
            (Join-Path $repositoryRoot "src\Citadel.WebApi\Dockerfile"),
            "--tag",
            $CoreImage,
            $repositoryRoot) `
        -Operation "Building the Core compatibility image"

    Invoke-CheckedNative `
        -FilePath "docker" `
        -Arguments @(
            "build",
            "--tag",
            $AgentImage,
            $AgentRepository) `
        -Operation "Building the Agent compatibility image"
}

Invoke-CheckedNative `
    -FilePath "dotnet" `
    -Arguments @(
        "build",
        $testProject,
        "--configuration",
        $Configuration) `
    -Operation "Building the acceptance test project"

$environmentVariables = @(
    "CITADEL_ACCEPTANCE_REQUIRE_CANDIDATE_IMAGES",
    "CITADEL_ACCEPTANCE_CORE_IMAGE",
    "CITADEL_ACCEPTANCE_AGENT_IMAGE",
    "CITADEL_ACCEPTANCE_AGENT_VERSION"
)
$previousEnvironment = @{}
foreach ($variable in $environmentVariables) {
    $previousEnvironment[$variable] =
        [Environment]::GetEnvironmentVariable(
            $variable,
            [EnvironmentVariableTarget]::Process)
}

try {
    $env:CITADEL_ACCEPTANCE_REQUIRE_CANDIDATE_IMAGES = "true"
    $env:CITADEL_ACCEPTANCE_CORE_IMAGE = $CoreImage
    $env:CITADEL_ACCEPTANCE_AGENT_IMAGE = $AgentImage
    if ([string]::IsNullOrWhiteSpace($ExpectedAgentVersion)) {
        Remove-Item Env:CITADEL_ACCEPTANCE_AGENT_VERSION `
            -ErrorAction SilentlyContinue
    }
    else {
        $env:CITADEL_ACCEPTANCE_AGENT_VERSION = $ExpectedAgentVersion
    }

    Invoke-CheckedNative `
        -FilePath "dotnet" `
        -Arguments @(
            "run",
            "--project",
            $testProject,
            "--no-build",
            "--configuration",
            $Configuration,
            "--",
            "-class",
            "Tests.Acceptance.Compatibility.RegularAgentCompatibilityTests") `
        -Operation "Running the Core and regular Agent compatibility suite"

    Invoke-CheckedNative `
        -FilePath "dotnet" `
        -Arguments @(
            "run",
            "--project",
            $testProject,
            "--no-build",
            "--configuration",
            $Configuration,
            "--",
            "-class",
            "Tests.Acceptance.Compatibility.EdgeAgentCompatibilityTests") `
        -Operation "Running the Core and Edge Agent compatibility suite"

    Invoke-CheckedNative `
        -FilePath "dotnet" `
        -Arguments @(
            "run",
            "--project",
            $testProject,
            "--no-build",
            "--configuration",
            $Configuration,
            "--",
            "-class",
            "Tests.Acceptance.Compatibility.SwarmNodeAgentCompatibilityTests") `
        -Operation "Running the multi-node Swarm node-agent compatibility suite"

    Invoke-CheckedNative `
        -FilePath "dotnet" `
        -Arguments @(
            "run",
            "--project",
            $testProject,
            "--no-build",
            "--configuration",
            $Configuration,
            "--",
            "-class",
            "Tests.Acceptance.Compatibility.RustFsBackupCompatibilityTests") `
        -Operation "Running the Core and RustFS compatibility suite"
}
finally {
    foreach ($variable in $environmentVariables) {
        [Environment]::SetEnvironmentVariable(
            $variable,
            $previousEnvironment[$variable],
            [EnvironmentVariableTarget]::Process)
    }
}
