using Citadel.Platforms.V1;
using Citadel.SharedModels.V1;
using Hosting.DockerClient.Models.Platforms;
using Infrastructure.Connectors.Mappers;
using Domain.Entities.Platforms;
using Google.Protobuf.WellKnownTypes;

namespace Tests.Unit.Infrastructure.Connectors;

public class PlatformMappersTests
{
    [Fact]
    public void Map_PlatformInfoResponse_Should_Create_Swarm_Descriptor()
    {
        var createdAt = DateTimeOffset.Parse("2026-08-03T10:00:00Z");
        var source = new PlatformInfoResponse
        {
            Id = "daemon-1",
            ApiVersion = "1.49",
            MinimumApiVersion = "1.24",
            SwarmInfo = new SwarmInfoMessage
            {
                NodeID = "node-1",
                NodeAddr = "10.0.0.10",
                LocalNodeState = "Active",
                ControlAvailable = true,
                Nodes = 3,
                Managers = 1,
                ClusterID = "cluster-1",
                ClusterCreatedAt = Timestamp.FromDateTimeOffset(createdAt),
                ServiceCount = 6,
                RunningTaskCount = 10
            }
        };

        var result = source.Map("swarm", "edge://swarm");

        var descriptor = Assert.IsType<DockerSwarmPlatformDescriptor>(result.Descriptor);
        Assert.Equal("cluster-1", descriptor.ClusterId);
        Assert.Equal(createdAt, descriptor.ClusterCreatedAt);
        Assert.Equal("1.49", descriptor.ApiVersion);
        Assert.True(descriptor.ControlAvailable);
        Assert.Equal(6, descriptor.ServiceCount);
        Assert.Equal(10, descriptor.RunningTaskCount);
        Assert.Equal("cluster-1", result.ClusterId);
    }

    [Fact]
    public void Map_PlatformInfoResult_Should_Create_Swarm_Descriptor()
    {
        var source = new PlatformInfoResult(
            Id: "daemon-1",
            CreatedAt: 0,
            NetworkCount: 1,
            VolumeCount: 2,
            AgentVersion: "test",
            ImageCount: 3,
            Driver: "overlay2",
            OperatingSystem: "Linux",
            OsVersion: "1",
            OsType: "linux",
            Architecture: "amd64",
            CpuCount: 4,
            MemoryTotal: 1024,
            ServerVersion: "28.0",
            ApiVersion: "1.49",
            MinimumApiVersion: "1.24",
            SwarmInfo: new SwarmInfoResult(
                NodeId: "node-1",
                NodeAddress: "10.0.0.10",
                LocalNodeState: "Active",
                ControlAvailable: true,
                Error: null,
                RemoteManagers: [],
                Nodes: 3,
                Managers: 1,
                ClusterId: "cluster-1",
                ServiceCount: 6,
                RunningTaskCount: 10));

        var result = source.Map("swarm", "local");

        var descriptor = Assert.IsType<DockerSwarmPlatformDescriptor>(result.Descriptor);
        Assert.Equal("cluster-1", descriptor.ClusterId);
        Assert.Equal("1.49", descriptor.ApiVersion);
        Assert.Equal(6, descriptor.ServiceCount);
        Assert.Equal(10, descriptor.RunningTaskCount);
        Assert.Equal("cluster-1", result.ClusterId);
    }

    [Fact]
    public void Map_PlatformStatsResponse_Should_Preserve_Normalized_Cpu_Usage()
    {
        var source = new PlatformStatsResponse
        {
            CpuCount = 12,
            MemTotal = 1024,
            AgentVersion = "test",
            ImageUsedBytes = 2048,
            VolumeUsedBytes = 4096,
            Stat = new PlatformStatMessage
            {
                CpuUsage = 0.25,
                MemoryUsage = 12,
                RxBytes = 1,
                TxBytes = 2,
                ContainerCount = 3,
                ContainersRunning = 2,
                ContainersPaused = 0,
                ContainersStopped = 1
            }
        };

        var result = source.Map();

        Assert.Equal(0.25, result.PlatformStat.CpuUsage);
        Assert.Null(result.PlatformStat.DiskUsedBytes);
        Assert.Null(result.PlatformStat.DiskTotalBytes);
        Assert.Null(result.PlatformStat.DiskUsage);
        Assert.Equal(2048, result.ImageUsedBytes);
        Assert.Equal(4096, result.VolumeUsedBytes);
    }

    [Fact]
    public void Map_PlatformStatsResponse_Should_Preserve_Disk_Field_Presence()
    {
        var source = new PlatformStatsResponse
        {
            Stat = new PlatformStatMessage
            {
                DiskUsedBytes = 0,
                DiskTotalBytes = 100,
                DiskUsage = 0
            }
        };

        var result = source.Map();

        Assert.Equal(0, result.PlatformStat.DiskUsedBytes);
        Assert.Equal(100, result.PlatformStat.DiskTotalBytes);
        Assert.Equal(0, result.PlatformStat.DiskUsage);
    }

    [Fact]
    public void Map_PlatformStreamResult_Should_Preserve_Normalized_Cpu_Usage()
    {
        var source = new PlatformStreamResult(
            NetworkCount: 1,
            VolumeCount: 1,
            ImageCount: 1,
            CpuCount: 12,
            MemoryTotal: 1024,
            AgentVersion: "test",
            ImageUsedBytes: 2048,
            VolumeUsedBytes: 4096,
            PlatformStatistics: new PlatformStatResult(
                MemoryUsage: 12,
                CpuUsage: 0.25,
                RxBytes: 1,
                TxBytes: 2,
                ContainerCount: 3,
                ContainersRunning: 2,
                ContainersPaused: 0,
                ContainersStopped: 1));

        var result = source.Map();

        Assert.Equal(0.25, result.PlatformStat.CpuUsage);
        Assert.Null(result.PlatformStat.DiskUsage);
        Assert.Equal(2048, result.ImageUsedBytes);
        Assert.Equal(4096, result.VolumeUsedBytes);
    }
}
