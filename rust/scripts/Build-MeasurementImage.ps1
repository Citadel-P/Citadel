param(
    [string]$Image = 'citadel-measurement:local'
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$artifactDirectory = Join-Path $repoRoot 'rust\artifacts'
New-Item -ItemType Directory -Force -Path $artifactDirectory | Out-Null

$watch = [System.Diagnostics.Stopwatch]::StartNew()
& docker build --file (Join-Path $repoRoot 'rust\Dockerfile') --tag $Image $repoRoot
$watch.Stop()
if ($LASTEXITCODE -ne 0) { throw 'measurement release image build failed.' }

$imageSize = [int64](& docker image inspect $Image --format '{{.Size}}')
$binarySize = [int64](& docker run --rm --entrypoint sh $Image -c 'stat -c %s /app/citadel-server')
$metadata = [ordered]@{
    measuredAtUtc = [DateTime]::UtcNow.ToString('O')
    image = $Image
    buildSeconds = [math]::Round($watch.Elapsed.TotalSeconds, 3)
    imageBytes = $imageSize
    binaryBytes = $binarySize
}
$metadata | ConvertTo-Json | Set-Content (Join-Path $artifactDirectory 'build.json')
$metadata | Format-List

