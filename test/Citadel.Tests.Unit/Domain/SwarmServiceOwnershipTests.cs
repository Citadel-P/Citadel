using Domain;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using System.Text.Json;

namespace Tests.Unit.Domain;

public sealed class SwarmServiceOwnershipTests
{
    [Fact]
    public void FromObservation_ShouldClassifyDockerStackService()
    {
        var projection = CreateProjection(new Dictionary<string, string>
        {
            ["com.docker.stack.namespace"] = "sample"
        });

        Assert.Equal(SwarmServiceOwnership.DockerStackExternal, projection.Ownership);
        Assert.Equal("sample", projection.DockerStackNamespace);
        Assert.Null(projection.OwnershipDiagnostic);
    }

    [Fact]
    public void FromObservation_ShouldTreatUnverifiedCitadelLabelsAsOrphanedMetadata()
    {
        var projection = CreateProjection(new Dictionary<string, string>
        {
            ["com.citadel.managed"] = "true",
            ["com.citadel.deployment-id"] = Guid.CreateVersion7().ToString()
        });

        Assert.Equal(SwarmServiceOwnership.Unmanaged, projection.Ownership);
        Assert.Equal("Orphaned Citadel metadata", projection.OwnershipDiagnostic);
    }

    [Fact]
    public void FromObservation_ShouldNotTrustCitadelOwnerIdWithoutManagedMarker()
    {
        var projection = CreateProjection(new Dictionary<string, string>
        {
            ["com.citadel.stack-id"] = Guid.CreateVersion7().ToString()
        });

        Assert.Equal(SwarmServiceOwnership.Unmanaged, projection.Ownership);
        Assert.Null(projection.OwnershipDiagnostic);
    }

    [Fact]
    public void PlatformJsonContext_ShouldSerializeOwnershipByName()
    {
        IReadOnlyList<SwarmServiceProjection> projections =
        [
            CreateProjection(new Dictionary<string, string>
            {
                ["com.docker.stack.namespace"] = "sample"
            })
        ];

        var json = JsonSerializer.Serialize(
            projections,
            PlatformJsonContext.Default.IReadOnlyListSwarmServiceProjection);
        using var document = JsonDocument.Parse(json);

        Assert.Equal(
            nameof(SwarmServiceOwnership.DockerStackExternal),
            document.RootElement[0].GetProperty("Ownership").GetString());
    }

    private static SwarmServiceProjection CreateProjection(IReadOnlyDictionary<string, string> labels) =>
        SwarmServiceProjection.FromObservation(
            Guid.CreateVersion7(),
            new SwarmServiceResult(
                "service-1", 1, "web", "Replicated", "nginx:latest", 1, 1,
                "Completed", null, [], [], [], [], labels, null, null),
            DateTimeOffset.UtcNow);
}
