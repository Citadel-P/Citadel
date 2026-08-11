using Application.Services;
using Domain.Contracts.Resources.Images;

namespace Tests.Unit.Application.Services;

public sealed class SwarmNodeAgentLifecycleServiceTests
{
    [Fact]
    public void GetLinuxArchitectures_ShouldIncludeSingleManifestDescriptorPlatform()
    {
        var distribution = new DistributionResult(
            new OCIDescriptorResult(
                "application/vnd.oci.image.manifest.v1+json",
                "sha256:test",
                1,
                new OCIPlatformResult("x86_64", "linux", string.Empty),
                string.Empty),
            []);

        var architectures = SwarmNodeAgentLifecycleService.GetLinuxArchitectures(distribution);

        Assert.Equal("amd64", Assert.Single(architectures));
    }

    [Fact]
    public void GetLinuxArchitectures_ShouldUnionManifestListAndDescriptorPlatforms()
    {
        var distribution = new DistributionResult(
            new OCIDescriptorResult(
                "application/vnd.oci.image.index.v1+json",
                "sha256:test",
                1,
                new OCIPlatformResult("amd64", "linux", string.Empty),
                string.Empty),
            [
                new OCIPlatformResult("aarch64", "linux", string.Empty),
                new OCIPlatformResult("amd64", "windows", string.Empty)
            ]);

        var architectures = SwarmNodeAgentLifecycleService.GetLinuxArchitectures(distribution);

        Assert.Equal(2, architectures.Count);
        Assert.Contains("amd64", architectures);
        Assert.Contains("arm64", architectures);
    }
}
