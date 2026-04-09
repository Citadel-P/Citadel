using Application.Configs;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Alerts;
using Domain.Entities.Deployments;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using System.Collections.Immutable;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class EventAlertTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> _platformConnectorFactoryMock = new();
    private readonly Mock<IConnectorFactory<IImageConnector>> _imageConnectorFactoryMock = new();
    private readonly Mock<IImageConnector> _imageConnectorMock = new();
    private readonly Mock<IOptions<JobConfiguration>> _configMock = new();
    private readonly Mock<IPlatformConnector> _platformConnector = new();
    private readonly Mock<IDelayWithJitterService> _delayWithJitter = new();
    private readonly Mock<IPlatformContainerCache> _platformCach = new();
    private readonly Mock<IAlertEventStreamManager> _alertEventStreamManager = new();

    private Func<CancellationToken, Task>? _runImageScannerJob;
    private Func<CancellationToken, Task>? _runAutoUpdateJob;

    private Guid _platformId;
    private Guid _alertRuleId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IOptions<JobConfiguration>>();
        services.RemoveAll<IDelayWithJitterService>();
        services.RemoveAll<IPlatformContainerCache>();
        services.RemoveAll<IAlertEventStreamManager>();

        services
            .AddHostedService<DeploymentImageScannerJob>()
            .AddHostedService<DeploymentAutoUpdateJob>()
            .AddHostedService<DbWriteWorker>()
            .AddHostedService<NotificationWorker>()
            .AddHostedService<AlertRuleCacheWarmup>();

        services.AddSingleton(_ => _configMock.Object);
        services.AddSingleton(_ => _platformConnector.Object);
        services.AddSingleton(_ => _platformConnectorFactoryMock.Object);
        services.AddSingleton(_ => _imageConnectorFactoryMock.Object);
        services.AddSingleton(_ => _delayWithJitter.Object);
        services.AddSingleton(_ => _platformCach.Object);
        services.AddSingleton(_ => _imageConnectorMock.Object);
        services.AddSingleton(_ => _alertEventStreamManager.Object);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration());
        var cacheEntry = new PlatformCacheEntry(_platformId, "localhost", PlatformConnectorType.Local, new Dictionary<string, Guid>().ToImmutableDictionary());
        var emptyError = Error.Empty as Error;
        _platformCach.Setup(x => x.TryGetCacheEntry(It.IsAny<Guid>(), out cacheEntry, out emptyError)).Returns(true);

        _imageConnectorFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_imageConnectorMock.Object);

        var dist = new DistributionResult(new OCIDescriptorResult("zip", "new-digest", 2000000, new OCIPlatformResult("x86", "linux", "6.2"), "art-1"));
        _imageConnectorMock.Setup(x => x.DistributionInspectAsync(It.IsAny<DistributionInspectCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult(Result.Success(dist)));

        _delayWithJitter
           .Setup(x => x.DelayWithJitterForAsync(It.IsAny<Func<CancellationToken, Task>>(),
                                                 It.IsAny<TimeSpan>(),
                                                 It.IsAny<CancellationToken>()))
           .Returns<Func<CancellationToken, Task>, TimeSpan, CancellationToken>((func, _, __) =>
           {
               if (func.Method.Name == "RunPeriodicScanAsync")
               {
                   _runImageScannerJob = func;
               }
               else if (func.Method.Name == "RunPeriodicAutoUpdate")
               {
                   _runAutoUpdateJob = func;
               }
               return Task.CompletedTask;
           });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        var deployment = new Deployment
        (
            name: "Test Deployment",
            description: "A deployment for testing",
            platformId: platform.Id,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec
            (
                UpdateBehavior: UpdateBehavior.Notify,
                Image: new ExternalImage
                (
                    RegistryId: Constants.DefaultRegistryId,
                    ImageTag: "nginx:latest",
                    ResolvedDigest: "sha256:old-digest"
                ),
                Ports: new List<string> { "80:80" },
                EnvVars: new List<string> { "ENV=production" }
            )

        );
        var container = new Container(
            platformId: platform.Id,
            dockerContainerId: "container-123",
            dockerImageId: "image",
            name: "deployment-container",
            created: 999999,
            state: ContainerStateStatus.Running,
            deploymentId: deployment.Id
        );
        var image = new Image(
            platformId: platform.Id,
            dockerImageId: "image",
            name: "deployment-image",
            tags: new List<string> { "nginx:latest" },
            containers: 1,
            size: 123456,
            createdAt: DateTime.UtcNow
        );

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await uow.Images.AddOrUpdateAsync(image, TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _platformId = platform.Id;
        _alertRuleId = (await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken))
            .Where(s => s.Type == AlertType.DeploymentImageUpdateAvailable).First().Id;
    }

    [Fact]
    public async Task EventAlert_ShouldTrigger_Immediately()
    {
        await EnsureAlertRuleCacheLoadedAsync(TestContext.Current.CancellationToken);
        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);

        await WaitForAlertEventCountAsync(1, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
        Assert.Equal(AlertType.DeploymentImageUpdateAvailable, alertEvents.Items.ElementAt(0).Type);
        await WaitForAlertStreamNotificationAsync(Times.Once(), TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.Is<AlertEvent>(a => a.Type == AlertType.DeploymentImageUpdateAvailable)), Times.Once);
        _alertEventStreamManager.Verify(x => x.SendUnresolvedAlertCount(1), Times.Once);
    }

    [Fact]
    public async Task EventAlert_ShouldNotRetrigger_DuringCooldown()
    {
        await UpdateAlertRuleAsync(rule =>
        {
            return AlertRule.FromPersistence(
                id: rule.Id,
                name: rule.Name,
                description: rule.Description,
                type: rule.Type,
                severity: rule.Severity,
                cooldownSeconds: 3600,
                status: AlertRuleStatus.Enabled,
                createdByActorId: rule.CreatedByActorId,
                createdAt: rule.CreatedAt,
                requiredMatches: rule.RequiredMatches,
                threshold: rule.Threshold,
                limitedTo: rule.LimitedTo,
                quietHours: rule.QuietHours);
        }, TestContext.Current.CancellationToken);

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await WaitForAlertEventCountAsync(1, TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        await WaitForAlertStreamNotificationAsync(Times.Once(), TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        await using var finalScope = Services.CreateAsyncScope();
        var db = finalScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>()), Times.Once);
        _alertEventStreamManager.Verify(x => x.SendUnresolvedAlertCount(1), Times.Once);
    }

    [Fact]
    public async Task EventAlert_ShouldNotTrigger_DuringQuietHours()
    {
        await UpdateAlertRuleAsync(rule =>
        {
            var now = DateTime.UtcNow;
            var quietHour = new DailyQuietHour("quiet", TimeOnly.FromDateTime(now.AddHours(-1)), TimeOnly.FromDateTime(now.AddHours(1)), "UTC", null);
            return AlertRule.FromPersistence(
                id: rule.Id,
                name: rule.Name,
                description: rule.Description,
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

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>()), Times.Never);
    }

    [Fact]
    public async Task EventAlert_ShouldNotTrigger_WhenScopeSpecificDoesNotMatch()
    {
        await UpdateAlertRuleAsync(rule =>
        {
            var limitedTo = new AlertRuleLimitedTo(AlertResourceType.Deployment, Guid.NewGuid());
            return AlertRule.FromPersistence(
                id: rule.Id,
                name: rule.Name,
                description: rule.Description,
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

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>()), Times.Never);
    }

    [Fact]
    public async Task EventAlert_ShouldNotTrigger_WhenDisabled()
    {
        await UpdateAlertRuleAsync(rule =>
        {
            rule.Disable();
            return rule;
        }, TestContext.Current.CancellationToken);

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>()), Times.Never);
    }

    private async Task RunAutoUpdateJobOnceAsync(CancellationToken cancellationToken)
    {
        var start = DateTime.UtcNow;
        while ((_runImageScannerJob is null || _runAutoUpdateJob is null) && DateTime.UtcNow - start < TimeSpan.FromSeconds(2))
        {
            await Task.Delay(50, cancellationToken);
        }

        if (_runImageScannerJob is null || _runAutoUpdateJob is null)
            throw new InvalidOperationException("Deployment image scanner or auto-update job was not initialized.");

        await RunScheduledJobOnceAsync(_runImageScannerJob, cancellationToken);
        await RunScheduledJobOnceAsync(_runAutoUpdateJob, cancellationToken);
    }

    private static async Task RunScheduledJobOnceAsync(Func<CancellationToken, Task> job, CancellationToken cancellationToken)
    {
        using var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        cts.CancelAfter(TimeSpan.FromSeconds(1));

        try
        {
            await job(cts.Token);
        }
        catch (OperationCanceledException) when (cts.IsCancellationRequested)
        {
        }
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

    private async Task WaitForAlertStreamNotificationAsync(Times times, TimeSpan timeout, CancellationToken cancellationToken)
    {
        var start = DateTime.UtcNow;

        while (DateTime.UtcNow - start < timeout)
        {
            try
            {
                _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>()), times);
                return;
            }
            catch (MockException)
            {
                await Task.Delay(100, cancellationToken);
            }
        }

        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>()), times);
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
}
