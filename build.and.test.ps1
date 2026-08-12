# docker build -f src/Citadel.WebApi/Dockerfile -t citadel-core:acceptance .
# docker build -t citadel-agent:acceptance ..\Citadel.Agent

$env:CITADEL_ACCEPTANCE_REQUIRE_CANDIDATE_IMAGES = "true"
$env:CITADEL_ACCEPTANCE_CORE_IMAGE = "citadel-core:acceptance"
$env:CITADEL_ACCEPTANCE_AGENT_IMAGE = "citadel-agent:acceptance"

dotnet run `
  --project test\Citadel.Tests.Acceptance\Citadel.Tests.Acceptance.csproj `
  -c Release -- `
  -class Tests.Acceptance.Compatibility.SwarmBackupCompatibilityTests