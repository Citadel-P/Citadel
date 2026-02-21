using Application.Configs;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using System.Threading.Channels;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ThresholdAlertTests: IntegrationTestBase
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> _platformFactoryMock = new();
    private readonly Mock<IOptions<JobConfiguration>> _configMock = new();
    private readonly Mock<IPlatformConnector> _platformConnector = new();
    private readonly TestPlatformHealthBroadCaster _broadcaster = new();
    private readonly Mock<IPlatformStreamManager> _streamManagerMock = new();
    private readonly Channel<(Guid Id, PlatformStatsResult Stats)> _channel = Channel.CreateUnbounded<(Guid Id, PlatformStatsResult Stats)>();

    private Guid _platformId;
    private Guid _alertRuleId;
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IOptions<JobConfiguration>>();

        services.AddHostedService<PlatformStatsWriterJob>();
        services.AddHostedService<PlatformStatsStreamerJob>();
        services.AddHostedService<DbWriteWorker>();
        services.AddHostedService<NotificationWorker>();
        services.AddHostedService<AlertRuleCacheWarmup>();
        services.AddSingleton(_channel);
        services.AddSingleton(_ => _streamManagerMock.Object);
        services.AddSingleton(_ => _configMock.Object);
        services.AddSingleton(_ => _platformConnector.Object);
        services.AddSingleton(_ => _platformFactoryMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(_ => _broadcaster);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var alertRule = new AlertRule(
            url: "http://example.com/alert",
            type: AlertType.PlatformCpuHigh,
            severity: AlertSeverity.Critical,
            cooldownSeconds: 60,
            isEnabled: true,
            scope: AlertScope.All,
            createdByActorId: Constants.DefaultAdminId,
            requiredMatches: 3,
            threshold: 85
        );

        var alertRuleState = new AlertRuleState(
            alertRuleId: alertRule.Id,
            resourceId: platform.Id,
            actorId: Constants.DefaultAdminId,
            3);
        
        await uow.AlertRules.AddAlertRuleAsync(alertRule, TestContext.Current.CancellationToken);
        await uow.AlertRules.AddAlertRuleStateAsync(alertRuleState, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _platformId = platform.Id;
        _alertRuleId = alertRule.Id;
    }

    [Fact]
    public async Task CpuHighAlert_ShouldTrigger_AfterThreeConsecutiveHighSamples()
    {
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration() { BatchSize = 3 });

        _platformFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_platformConnector.Object);

        _platformConnector
            .Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        await _broadcaster.PublishAsync(
            new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, true),
            TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(_platformId, TestContext.Current.CancellationToken);
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Equal(3, stats.Count());
        Assert.Single(alertEvents.Items);
    }

    [Fact]
    public async Task CpuHighAlert_ShouldNotTrigger_WhenBelowThreshold()
    {
        _platformFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_platformConnector.Object);

        _platformConnector
            .Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns(() => GetLowCpuStatsAsync());

        await _broadcaster.PublishAsync(
            new PlatformHealth(_platformId, "addr", PlatformConnectorType.Agent, true),
            TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
    }

    [Fact]
    public async Task CpuHighAlert_ShouldNotTrigger_WhenSequenceIsBroken()
    {
        _platformFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_platformConnector.Object);

        _platformConnector
            .Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns(() => GetBrokenSequenceStatsAsync());

        await _broadcaster.PublishAsync(
            new PlatformHealth(_platformId, "addr", PlatformConnectorType.Agent, true),
            TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
    }

    [Fact]
    public async Task CpuHighAlert_ShouldNotRetrigger_DuringCooldown()
    {
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration { BatchSize = 3 });

        _platformFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_platformConnector.Object);

        _platformConnector
            .Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns(() => GetStatsAsync());

        // Ensure clean counter
        await using (var scope = Services.CreateAsyncScope())
        {
            var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var state = await db.AlertRules.GetStateAsync(_alertRuleId, _platformId, TestContext.Current.CancellationToken);
            state!.Reset();
            await db.AlertRules.UpdateStateAsync(state, TestContext.Current.CancellationToken);
            await db.CommitAsync(TestContext.Current.CancellationToken);
        }

        // First trigger
        await _broadcaster.PublishAsync(
            new PlatformHealth(_platformId, "addr", PlatformConnectorType.Agent, true),
            TestContext.Current.CancellationToken);

        await Task.Delay(1500, TestContext.Current.CancellationToken);

        // Second trigger (still within cooldown)
        await _broadcaster.PublishAsync(
            new PlatformHealth(_platformId, "addr", PlatformConnectorType.Agent, true),
            TestContext.Current.CancellationToken);

        await Task.Delay(1500, TestContext.Current.CancellationToken);

        await using var finalScope = Services.CreateAsyncScope();
        var dbFinal = finalScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await dbFinal.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
    }

    [Fact]
    public async Task DisabledAlertRule_ShouldNotTrigger()
    {
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var rule = await db.AlertRules.GetByIdAsync(_alertRuleId, TestContext.Current.CancellationToken);
        rule?.Disable();
        await db.AlertRules.UpdateAsync(rule, TestContext.Current.CancellationToken);
        await db.CommitAsync(TestContext.Current.CancellationToken);

        await _broadcaster.PublishAsync(
            new PlatformHealth(_platformId, "addr", PlatformConnectorType.Agent, true),
            TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken);

        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);
        Assert.Empty(alertEvents.Items);
    }

    private static async IAsyncEnumerable<PlatformStatsResult> GetLowCpuStatsAsync()
    {
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        yield return BuildStat(time, 50);
        await Task.Delay(50);
        yield return BuildStat(time + 60, 60);
        await Task.Delay(50);
        yield return BuildStat(time + 120, 70);
    }

    private static async IAsyncEnumerable<PlatformStatsResult> GetBrokenSequenceStatsAsync()
    {
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        yield return BuildStat(time, 90);
        await Task.Delay(50);
        yield return BuildStat(time + 60, 40); // breaks sequence
        await Task.Delay(50);
        yield return BuildStat(time + 120, 95);
    }

    private static PlatformStatsResult BuildStat(long time, int cpu)
    {
        return new PlatformStatsResult(
            123456, 5, 2, 1,
            new DockerPlatformStat(
                created: time,
                memoryUsage: 500,
                cpuUsage: cpu,
                rxBytes: 100,
                txBytes: 200,
                containerCount: 3,
                containersPaused: 0,
                containersStopped: 1,
                containersRunning: 2));
    }

    private static async IAsyncEnumerable<PlatformStatsResult> GetStatsAsync()
    {
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        {
            var stat1 = new PlatformStatsResult
            (
                MemTotal: 123456,
                ImageCount: 5,
                VolumeCount: 2,
                NetworkCount: 1,
                PlatformStat: new DockerPlatformStat
                (
                    created: time,
                    memoryUsage: 500,
                    cpuUsage: 86,
                    rxBytes: 100,
                    txBytes: 200,
                    containerCount: 3,
                    containersPaused: 0,
                    containersStopped: 1,
                    containersRunning: 2
                )
            );

            yield return stat1;
        }
        await Task.Delay(50);
        {
            var stat2 = new PlatformStatsResult
            (
                MemTotal: 123456,
                ImageCount: 6,
                VolumeCount: 3,
                NetworkCount: 10,
                PlatformStat: new DockerPlatformStat
                (
                    created: time + (60 * 2),
                    memoryUsage: 800,
                    cpuUsage: 87,
                    rxBytes: 300,
                    txBytes: 400,
                    containerCount: 5,
                    containersPaused: 1,
                    containersStopped: 1,
                    containersRunning: 3
                )
            );
            yield return stat2;
        }
        await Task.Delay(50);
        {
            var stat2 = new PlatformStatsResult
            (
                MemTotal: 123456,
                ImageCount: 6,
                VolumeCount: 3,
                NetworkCount: 10,
                PlatformStat: new DockerPlatformStat
                (
                    created: time + (60 * 3),
                    memoryUsage: 800,
                    cpuUsage: 88,
                    rxBytes: 300,
                    txBytes: 400,
                    containerCount: 5,
                    containersPaused: 1,
                    containersStopped: 1,
                    containersRunning: 3
                )
            );
            yield return stat2;
        }
    }
}
