using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Hosting.Common;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class ContainerLogStreamManagerTests
{
    [Fact]
    public void StartContainerLogs_FullDockerId_UsesNormalizedSubscriberGroup()
    {
        const string fullContainerId = "a4c05df3937c5d2479d48cbf6b15eb85e719c27c30205fc7c9b2f82b9b973750";
        const string normalizedContainerId = "a4c05df3937c";
        var cache = new Mock<IPlatformContainerCache>();
        PlatformCacheEntry? cacheEntry = null;
        cache
            .Setup(x => x.TryGetPlatformWithContainer(normalizedContainerId, out cacheEntry!))
            .Returns(false);

        var manager = new ContainerLogStreamManager(
            Mock.Of<IApplicationHubDispatcher>(),
            Mock.Of<ILogger<ContainerLogStreamManager>>(),
            cache.Object,
            new ContainerEventBroadcaster(),
            Mock.Of<IConnectorFactory<IContainerConnector>>());
        var groupId = Constants.WellKnownSignalRGroups.ContainerLogGroup(normalizedContainerId);
        manager.AddSubscriber(groupId, "connection-1");

        manager.StartContainerLogs(fullContainerId);

        cache.Verify(
            x => x.TryGetPlatformWithContainer(normalizedContainerId, out cacheEntry!),
            Times.Once);
        manager.RemoveSubscriber(groupId, "connection-1");
    }
}
