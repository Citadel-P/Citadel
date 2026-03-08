using Application.Configs;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Alerts;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
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
        services.AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Reader);
        services.AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Writer);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        _alertRuleId = (await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken))
            .Where(s => s.Type == AlertType.PlatformCpuHigh).First().Id;

        var alertRuleState = new AlertRuleState(
            alertRuleId: _alertRuleId,
            resourceId: platform.Id,
            actorId: Constants.DefaultAdminId,
            3);
        
        await uow.AlertRules.UpsertAlertRuleStateAsync(alertRuleState, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _platformId = platform.Id;
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
            await db.AlertRules.UpsertAlertRuleStateAsync(state, TestContext.Current.CancellationToken);
            await db.CommitAsync(TestContext.Current.CancellationToken);
        }

        // First trigger
        await _broadcaster.PublishAsync(
            new PlatformHealth(_platformId, "addr", PlatformConnectorType.Agent, true),
            TestContext.Current.CancellationToken);

        await WaitForAlertEventCountAsync(1, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

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
    public async Task CpuHighAlert_ShouldNotTrigger_DuringQuietHours()
    {
        await UpdateAlertRuleAsync(rule =>
        {
            var now = DateTime.UtcNow;
            var quietHour = new DailyQuietHour("quiet", TimeOnly.FromDateTime(now.AddHours(-1)), TimeOnly.FromDateTime(now.AddHours(1)), "UTC", null);
            return AlertRule.FromPersistence(
                id: rule.Id,
                name: rule.Name,
                type: rule.Type,
                severity: rule.Severity,
                cooldownSeconds: rule.CooldownSeconds,
                status: AlertRuleStatus.Enabled,
                createdByActorId: rule.CreatedByActorId,
                createdAt: rule.CreatedAt,
                requiredMatches: rule.RequiredMatches,
                threshold: rule.Threshold,
                limitedTo: rule.LimitedTo,
                quietHours: [quietHour]);
        }, TestContext.Current.CancellationToken);

        _platformFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_platformConnector.Object);

        _platformConnector
            .Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns(() => GetStatsAsync());

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
    public async Task CpuHighAlert_ShouldNotTrigger_WhenScopeSpecificDoesNotMatch()
    {
        await UpdateAlertRuleAsync(rule =>
        {
            var limitedTo = new AlertRuleLimitedTo(AlertResourceType.Platform, Guid.NewGuid());
            return AlertRule.FromPersistence(
                id: rule.Id,
                name: rule.Name,
                type: rule.Type,
                severity: rule.Severity,
                cooldownSeconds: rule.CooldownSeconds,
                status: AlertRuleStatus.Enabled,
                createdByActorId: rule.CreatedByActorId,
                createdAt: rule.CreatedAt,
                requiredMatches: rule.RequiredMatches,
                threshold: rule.Threshold,
                limitedTo: [limitedTo],
                quietHours: rule.QuietHours);
        }, TestContext.Current.CancellationToken);

        _platformFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_platformConnector.Object);

        _platformConnector
            .Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns(() => GetStatsAsync());

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
    public async Task CpuHighAlert_ShouldRequireNewMatches_AfterTrigger()
    {
        await EnsureAlertRuleCacheLoadedAsync(TestContext.Current.CancellationToken);
        await using (var scope = Services.CreateAsyncScope())
        {
            var context = BuildCpuContext([91, 92, 93]);
            await RunAlertInlineAsync(scope.ServiceProvider, context, TestContext.Current.CancellationToken);
        }

        await WaitForAlertEventCountAsync(1, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        await using (var scope = Services.CreateAsyncScope())
        {
            var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var rule = await db.AlertRules.GetByIdAsync(_alertRuleId, TestContext.Current.CancellationToken);
            var state = await db.AlertRules.GetStateAsync(_alertRuleId, _platformId, TestContext.Current.CancellationToken);
            var updatedState = AlertRuleState.FromPersistence(
                alertRuleId: state!.AlertRuleId,
                resourceId: state.ResourceId,
                consecutiveMatches: 0,
                lastTriggeredAt: DateTime.UtcNow.AddSeconds(-(rule!.CooldownSeconds.Value + 1)),
                createdByActorId: state.CreatedByActorId,
                createdAt: state.CreatedAt);
            await db.AlertRules.UpsertAlertRuleStateAsync(updatedState, TestContext.Current.CancellationToken);
            await db.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var context = BuildCpuContext([91, 92]);
            await RunAlertInlineAsync(scope.ServiceProvider, context, TestContext.Current.CancellationToken);
        }

        await Task.Delay(500, TestContext.Current.CancellationToken);

        await using var finalScope = Services.CreateAsyncScope();
        var dbFinal = finalScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await dbFinal.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
    }

    private static async Task RunAlertInlineAsync(IServiceProvider services, AlertEvaluationContext context, CancellationToken cancellationToken)
    {
        var uow = services.GetRequiredService<IUnitOfWork>();
        var evaluators = services.GetRequiredService<IEnumerable<IAlertEvaluator>>();
        var notificationQueue = services.GetRequiredService<INotificationQueue>();
        var ruleProvider = services.GetRequiredService<IAlertRuleProvider>();
        var notificationService = services.GetRequiredService<INotificationRepository>();
        var logger = services.GetRequiredService<ILogger<AlertService>>();

        var queue = new InlineDbWorkQueue(uow);
        var alertService = new AlertService(queue, notificationQueue, ruleProvider, evaluators, notificationService, logger);

        await alertService.ProcessAsync(AlertType.PlatformCpuHigh, context, cancellationToken);
    }

    private sealed class InlineDbWorkQueue(IUnitOfWork uow) : IDbWorkQueue
    {
        private readonly Channel<IDbWorkItem> _channel = Channel.CreateUnbounded<IDbWorkItem>();

        public ChannelReader<IDbWorkItem> Reader => _channel.Reader;

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
            => new(item.ExecuteAsync(uow, cancellationToken));
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
            123456, 5, 2, 1, "1.0",
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
                AgentVersion: "1.0",
                PlatformStat: new DockerPlatformStat
                (
                    created: time,
                    memoryUsage: 50,
                    cpuUsage: 91,
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
                AgentVersion: "1.0",
                PlatformStat: new DockerPlatformStat
                (
                    created: time + (60 * 2),
                    memoryUsage: 60,
                    cpuUsage: 92,
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
                AgentVersion: "1.0",
                PlatformStat: new DockerPlatformStat
                (
                    created: time + (60 * 3),
                    memoryUsage: 70,
                    cpuUsage: 93,
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

    private AlertEvaluationContext BuildCpuContext(IEnumerable<double> cpuUsages)
    {
        var snapshots = cpuUsages
            .Select(cpu => new PlatformAlertSnapshot(_platformId, "platform", cpu, 0, string.Empty))
            .ToArray();

        return new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: snapshots,
            Deployments: [],
            Stacks: []);
    }

    private async Task<int> WaitForAlertEventCountAsync(int expectedCount, TimeSpan timeout, CancellationToken cancellationToken)
    {
        var start = DateTime.UtcNow;
        var count = 0;

        while (DateTime.UtcNow - start < timeout)
        {
            await using var scope = Services.CreateAsyncScope();
            var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, cancellationToken);
            count = alertEvents.Items.Count();

            if (count >= expectedCount)
                break;

            await Task.Delay(200, cancellationToken);
        }

        return count;
    }

    private async Task UpdateAlertRuleAsync(Func<AlertRule, AlertRule> update, CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<AlertRuleCache>();
        var rule = await db.AlertRules.GetByIdAsync(_alertRuleId, cancellationToken);
        var updated = update(rule!);
        await db.AlertRules.UpdateAsync(updated, cancellationToken);
        await db.CommitAsync(cancellationToken);
        await cache.ReloadAsync(cancellationToken);
    }

    private async Task EnsureAlertRuleCacheLoadedAsync(CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var cache = scope.ServiceProvider.GetRequiredService<AlertRuleCache>();
        await cache.ReloadAsync(cancellationToken);
    }

    [Fact]
    public async Task CpuHighAlert_ShouldOnlyFireMostSevere_WhenMultipleRulesMatch()
    {
        // Existing seeded rule:
        // - PlatformCpuHigh, threshold=90, severity=Critical, requiredMatches=3
        // - PlatformCpuHigh, threshold=80, severity=Warning, requiredMatches=3
        // Add a third rule: PlatformCpuHigh, threshold=75, severity=Critical, requiredMatches=3
        await AddSecondCpuHighRuleAsync(
            threshold: 75,
            severity: AlertSeverity.Critical,
            requiredMatches: 3,
            cooldownSeconds: 300,
            TestContext.Current.CancellationToken);

        await EnsureAlertRuleCacheLoadedAsync(TestContext.Current.CancellationToken);

        // CPU=95 exceeds both thresholds (75 and 90) — only Critical should fire
        await using (var scope = Services.CreateAsyncScope())
        {
            var context = BuildCpuContext([95, 95, 95]);
            await RunAlertInlineAsync(scope.ServiceProvider, context, TestContext.Current.CancellationToken);
        }

        await WaitForAlertEventCountAsync(1, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        await using var finalScope = Services.CreateAsyncScope();
        var db = finalScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
        Assert.Equal(AlertSeverity.Critical, alertEvents.Items.First().Severity);
    }

    [Fact]
    public async Task CpuHighAlert_ShouldFireLowerSeverity_WhenHigherDoesNotMatch()
    {
        // Existing seeded rule:
        // - PlatformCpuHigh, threshold=90, severity=Critical, requiredMatches=3
        // - PlatformCpuHigh, threshold=80, severity=Warning, requiredMatches=3
        // Add a third rule: PlatformCpuHigh, threshold=95, severity=Critical, requiredMatches=3
        await AddSecondCpuHighRuleAsync(
            threshold: 95,
            severity: AlertSeverity.Critical,
            requiredMatches: 3,
            cooldownSeconds: 300,
            TestContext.Current.CancellationToken);

        await EnsureAlertRuleCacheLoadedAsync(TestContext.Current.CancellationToken);

        // CPU=92 exceeds only the Warning threshold (90), not Critical (95)
        await using (var scope = Services.CreateAsyncScope())
        {
            var context = BuildCpuContext([92, 92, 92]);
            await RunAlertInlineAsync(scope.ServiceProvider, context, TestContext.Current.CancellationToken);
        }

        await WaitForAlertEventCountAsync(1, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        await using var finalScope = Services.CreateAsyncScope();
        var db = finalScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
        Assert.Equal(AlertSeverity.Critical, alertEvents.Items.First().Severity);
        Assert.Equal(_alertRuleId, alertEvents.Items.First().AlertRuleId);
    }

    private async Task<Guid> AddSecondCpuHighRuleAsync(
        double threshold,
        AlertSeverity severity,
        int requiredMatches,
        int cooldownSeconds,
        CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var rule = new AlertRule(
            type: AlertType.PlatformCpuHigh,
            name: "CPU Usage Above " + threshold,
            severity: severity,
            cooldownSeconds: cooldownSeconds,
            status: AlertRuleStatus.Enabled,
            createdByActorId: Constants.DefaultAdminId,
            requiredMatches: requiredMatches,
            threshold: threshold);

        var state = new AlertRuleState(
            alertRuleId: rule.Id,
            resourceId: _platformId,
            actorId: Constants.DefaultAdminId,
            consecutiveMatches: requiredMatches);

        await db.AlertRules.AddAlertRuleAsync(rule, cancellationToken);
        await db.AlertRules.UpsertAlertRuleStateAsync(state, cancellationToken);
        await db.CommitAsync(cancellationToken);

        return rule.Id;
    }
}
