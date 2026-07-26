using Citadel.Platforms.V1;
using Citadel.SharedModels.V1;
using Hosting.DockerClient.Models.Platforms;
using Infrastructure.Connectors.Mappers;

namespace Tests.Unit.Infrastructure.Connectors;

public class PlatformMappersTests
{
    [Fact]
    public void Map_PlatformStatsResponse_Should_Preserve_Normalized_Cpu_Usage()
    {
        var source = new PlatformStatsResponse
        {
            CpuCount = 12,
            MemTotal = 1024,
            AgentVersion = "test",
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
    }
}
