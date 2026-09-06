# Test suites own a unique target volume. Registry/toolchain caches stay shared.
function New-CitadelBuildCacheName {
    return "citadel-rust-test-$([Guid]::NewGuid().ToString('N'))"
}

function Remove-CitadelBuildCache {
    param([Parameter(Mandatory)][string]$Name)

    if ($Name -cnotmatch '^citadel-rust-test-[a-f0-9]{32}$') {
        throw "Refusing to remove a non-test build cache: $Name"
    }
    $existing = @(& docker volume ls --quiet --filter "name=^${Name}$")
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Docker is unavailable; build cache $Name could not be cleaned."
        return
    }
    if ($existing -ccontains $Name) {
        # No force/prune: Docker must refuse removal if a container still uses it.
        & docker volume rm $Name | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "Build cache $Name is still in use or could not be removed."
        }
    }
}
