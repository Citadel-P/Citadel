using Domain;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace Tests.Integration.WebApi;

public sealed class SwarmCapabilityMapperTests
{
    [Fact]
    public void ToSwarmCapabilities_ShouldReturnNullForStandalonePlatform()
    {
        var platform = CreatePlatform(new DockerPlatformDescriptor("daemon-1", 0, 0, 0, 0));

        Assert.Null(CapabilityMapper.ToSwarmCapabilities(
            platform,
            new PermissionMetadata(PermissionLevel.Execute, SpecificPermission.None)));
    }

    [Fact]
    public void ToSwarmCapabilities_ShouldKeepReadOnlyViewsAvailableWhileOffline()
    {
        var platform = CreatePlatform(CreateSwarmDescriptor(apiVersion: "1.49"), PlatformStatus.Offline);

        var capabilities = Assert.IsType<SwarmCapabilities>(CapabilityMapper.ToSwarmCapabilities(
            platform,
            new PermissionMetadata(PermissionLevel.Execute, SpecificPermission.None)));

        Assert.True(capabilities.CanViewCluster);
        Assert.True(capabilities.CanViewServices);
        Assert.False(capabilities.CanManageNodes);
        Assert.False(capabilities.CanManageNetworks);
        Assert.False(capabilities.SupportsServiceStatus);
    }

    [Fact]
    public void ToSwarmCapabilities_ShouldAdvertiseImplementedMutations()
    {
        var platform = CreatePlatform(CreateSwarmDescriptor("1.49"));

        var capabilities = Assert.IsType<SwarmCapabilities>(CapabilityMapper.ToSwarmCapabilities(
            platform,
            new PermissionMetadata(PermissionLevel.Write, SpecificPermission.None)));

        Assert.False(capabilities.SupportsServiceStatus);
        Assert.True(capabilities.CanManageNodes);
        Assert.True(capabilities.CanManageNetworks);
        Assert.True(capabilities.CanManageSecrets);
        Assert.True(capabilities.CanManageConfigs);
        Assert.False(capabilities.SupportsDeploymentApply);
        Assert.False(capabilities.SupportsStackApply);
        Assert.False(capabilities.SupportsClusterVolumes);
    }

    private static Platform CreatePlatform(
        PlatformDescriptor descriptor,
        PlatformStatus status = PlatformStatus.Online) => new(
            "platform", "unix:///var/run/docker.sock", 0, 0, 0, 4, 1024,
            "28.0", null, status, PlatformConnectorType.Local, descriptor);

    private static DockerSwarmPlatformDescriptor CreateSwarmDescriptor(string? apiVersion) => new(
        "node-1", "10.0.0.1", "Active", true, 1, 1, "daemon-1", 0, 0, 0, 0,
        ApiVersion: apiVersion,
        ClusterId: "cluster-1");
}
