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
using System;
using System.Collections.Generic;
using System.Text;
using System.Threading.Channels;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class AlerterTests: IntegrationTestBase
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
                url : "http://example.com/alert",
                type: AlertType.PlatformCpuHigh,
                cooldownSeconds: 60,
                threshold: 85,
                requiredMatches: 3,
                isEnabled: true,
                scope: AlertScope.All,
                createdByActorId: Constants.DefaultAdminId,
                severity: AlertSeverity.Critical
            );

        var alertRuleState = new AlertRuleState(
            alertRuleId: alertRule.Id,
            resourceId: platform.Id,
            actorId: Constants.DefaultAdminId,
            3);
        
        await uow.Alerters.AddAlertRuleAsync(alertRule, TestContext.Current.CancellationToken);
        await uow.Alerters.AddAlertRuleStateAsync(alertRuleState, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _platformId = platform.Id;
        _alertRuleId = alertRule.Id;
    }

    [Fact]
    public async Task RaiseAlert_WhenCpuReachesThreshold_3_ConsecutiveTime()
    {
        // Arrange
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration() { BatchSize = 3 });

        _platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(_platformConnector.Object);
        _platformConnector.Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        // Act
        await _broadcaster.PublishAsync(new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken); // wait for jobs to process

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(_platformId, TestContext.Current.CancellationToken);

        Assert.Equal(3, stats.Count());
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
