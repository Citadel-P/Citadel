using Application.Services.Abstractions;
using Application.Services.SignalR;
using Hosting.Common;
using Moq;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class StatsStreamManagerTests
{
    private readonly Mock<IApplicationHubDispatcher> _dispatcher = new();

    [Fact]
    public void ContainerManager_ShouldReportSubscribersOnlyForTheRequestedPlatform()
    {
        var manager = new ContainerStreamManager(_dispatcher.Object);
        var subscribedPlatformId = Guid.CreateVersion7();
        var otherPlatformId = Guid.CreateVersion7();

        manager.AddSubscriber(
            Constants.WellKnownSignalRGroups.ContainersGroup(subscribedPlatformId),
            "connection-1");

        Assert.True(manager.HasStatsSubscribers(subscribedPlatformId));
        Assert.False(manager.HasStatsSubscribers(otherPlatformId));
    }

    [Fact]
    public void PlatformManager_ShouldReportStatsSubscribersForThePlatformsGroup()
    {
        var manager = new PlatformStreamManager(_dispatcher.Object);

        Assert.False(manager.HasStatsSubscribers);

        manager.AddSubscriber(Constants.WellKnownSignalRGroups.PlatformsGroup, "connection-1");

        Assert.True(manager.HasStatsSubscribers);
    }
}
