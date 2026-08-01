using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class PlatformStatsWriterDiskTests
{
    [Fact]
    public void BuildThresholdAlertSnapshot_ShouldUseNewestDiskSampleOnly()
    {
        var samples = new[]
        {
            CreateSample(diskUsage: 75, diskUsedBytes: 75, diskTotalBytes: 100),
            CreateSample(diskUsage: null, diskUsedBytes: null, diskTotalBytes: null)
        };

        var snapshot = PersistPlatformStatsWorkItem.BuildThresholdAlertSnapshot(
            Guid.CreateVersion7(),
            "docker-01",
            samples);

        Assert.Null(snapshot.DiskUsage);
        Assert.Null(snapshot.DiskUsedBytes);
        Assert.Null(snapshot.DiskTotalBytes);
    }

    [Fact]
    public void BuildThresholdAlertSnapshot_ShouldPreserveNewestCompleteDiskSample()
    {
        var snapshot = PersistPlatformStatsWorkItem.BuildThresholdAlertSnapshot(
            Guid.CreateVersion7(),
            "docker-01",
            [CreateSample(diskUsage: 90, diskUsedBytes: 90, diskTotalBytes: 100)]);

        Assert.Equal(90, snapshot.DiskUsage);
        Assert.Equal(90, snapshot.DiskUsedBytes);
        Assert.Equal(100, snapshot.DiskTotalBytes);
    }

    [Theory]
    [InlineData(-0.01)]
    [InlineData(100.01)]
    [InlineData(double.NaN)]
    [InlineData(double.PositiveInfinity)]
    public void BuildThresholdAlertSnapshot_ShouldRejectInvalidDiskPercentage(double usage)
    {
        var snapshot = PersistPlatformStatsWorkItem.BuildThresholdAlertSnapshot(
            Guid.CreateVersion7(),
            "docker-01",
            [CreateSample(diskUsage: usage, diskUsedBytes: 70, diskTotalBytes: 100)]);

        Assert.Null(snapshot.DiskUsage);
        Assert.Null(snapshot.DiskUsedBytes);
        Assert.Null(snapshot.DiskTotalBytes);
    }

    [Fact]
    public async Task PersistPlatformStatsWorkItem_ShouldPropagatePersistenceFailure()
    {
        var platformId = Guid.CreateVersion7();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(
                platformId,
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("database unavailable"));
        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(value => value.Platforms).Returns(platforms.Object);
        var workItem = new PersistPlatformStatsWorkItem(
            new Dictionary<Guid, List<PlatformStatsResult>>
            {
                [platformId] =
                [
                    CreateSample(
                        diskUsage: 50,
                        diskUsedBytes: 50,
                        diskTotalBytes: 100)
                ]
            });

        var exception = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            workItem.ExecuteAsync(uow.Object, TestContext.Current.CancellationToken));

        Assert.Equal("database unavailable", exception.Message);
    }

    private static PlatformStatsResult CreateSample(
        double? diskUsage,
        long? diskUsedBytes,
        long? diskTotalBytes)
        => new(
            MemTotal: 1024,
            ImageCount: 1,
            VolumeCount: 1,
            NetworkCount: 1,
            AgentVersion: "1.0.0",
            PlatformStat: new DockerPlatformStat(
                created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                memoryUsage: 20,
                cpuUsage: 10,
                rxBytes: 1,
                txBytes: 2,
                containerCount: 1,
                containersPaused: 0,
                containersStopped: 0,
                containersRunning: 1,
                diskUsedBytes: diskUsedBytes,
                diskTotalBytes: diskTotalBytes,
                diskUsage: diskUsage));
}
