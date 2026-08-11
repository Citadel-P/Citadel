using Application.Features.Swarm.Queries;
using Domain;
using Domain.Entities.Platforms;

namespace Tests.Unit.Application.Features.Swarm;

public sealed class SwarmQuorumResultTests
{
    [Theory]
    [InlineData(PlatformStatus.Online, 1, 1, true, false, SwarmQuorumState.Healthy, 1)]
    [InlineData(PlatformStatus.Online, 3, 2, true, false, SwarmQuorumState.Degraded, 2)]
    [InlineData(PlatformStatus.Online, 3, 1, true, false, SwarmQuorumState.Lost, 2)]
    [InlineData(PlatformStatus.Online, 3, 3, false, false, SwarmQuorumState.Lost, 2)]
    [InlineData(PlatformStatus.Online, 3, 3, true, true, SwarmQuorumState.Unknown, 2)]
    [InlineData(PlatformStatus.Offline, 3, 3, true, false, SwarmQuorumState.Unknown, 2)]
    [InlineData(PlatformStatus.Online, 0, 0, false, false, SwarmQuorumState.Unknown, 0)]
    public void Calculate_ShouldReportManagerMajorityAndLeaderState(
        PlatformStatus connectionStatus,
        int managerCount,
        int reachableManagerCount,
        bool hasLeader,
        bool isManagerInventoryStale,
        SwarmQuorumState expectedState,
        int expectedRequiredManagers)
    {
        var summary = new SwarmProjectionSummary(
            IsStale: isManagerInventoryStale,
            NodeCount: managerCount,
            ManagerCount: managerCount,
            ReachableManagerCount: reachableManagerCount,
            HasLeader: hasLeader,
            IsManagerInventoryStale: isManagerInventoryStale,
            ServiceCount: 0,
            ServiceStatusCounts: PlatformWorkloadStatusCounts.Empty,
            RunningTaskCount: 0,
            DesiredTaskCount: 0,
            NetworkCount: 0,
            LocalNetworkCount: 0,
            VolumeCount: 0,
            ImageCount: 0);

        var result = SwarmQuorumResult.Calculate(connectionStatus, summary);

        Assert.Equal(expectedState, result.State);
        Assert.Equal(reachableManagerCount, result.ReachableManagers);
        Assert.Equal(expectedRequiredManagers, result.RequiredManagers);
        Assert.Equal(hasLeader, result.HasLeader);
    }
}
