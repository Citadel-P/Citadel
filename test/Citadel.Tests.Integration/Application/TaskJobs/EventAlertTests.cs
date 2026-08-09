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
    private readonly ObservableDbWorkQueue _dbWorkQueue = new();
    private readonly TaskCompletionSource _alertNotification =
        new(TaskCreationOptions.RunContinuationsAsynchronously);

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
        services.RemoveAll<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue>(_dbWorkQueue);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration());
        var cacheEntry = new PlatformCacheEntry(_platformId, "localhost", PlatformConnectorType.Local, new Dictionary<string, Guid>().ToImmutableDictionary());
        var emptyError = Error.Empty as Error;
        _platformCach.Setup(x => x.TryGetCacheEntry(It.IsAny<Guid>(), out cacheEntry, out emptyError)).Returns(true);

        _imageConnectorFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(_imageConnectorMock.Object);

        var dist = new DistributionResult(new OCIDescriptorResult("zip", "new-digest", 2000000, new OCIPlatformResult("x86", "linux", "6.2"), "art-1"));
        _imageConnectorMock.Setup(x => x.DistributionInspectAsync(It.IsAny<DistributionInspectCommand>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult(Result.Success(dist)));
        _alertEventStreamManager
            .Setup(x => x.SendTriggeredAlertEvent(
                It.IsAny<AlertEvent>(),
                It.IsAny<IEnumerable<Guid>>()))
            .Callback(() => _alertNotification.TrySetResult())
            .Returns(Task.CompletedTask);

        _delayWithJitter
           .Setup(x => x.DelayWithJitterForAsync(It.IsAny<Func<CancellationToken, Task>>(),
                                                 It.IsAny<TimeSpan>(),
                                                 It.IsAny<CancellationToken>()))
           .Returns(Task.CompletedTask);
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
                Ports: new List<string> { "80:80" }
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

        Assert.Equal(1, await GetAlertEventCountAsync(TestContext.Current.CancellationToken));

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
        Assert.Equal(AlertType.DeploymentImageUpdateAvailable, alertEvents.Items.ElementAt(0).Type);
        await WaitForAlertStreamNotificationAsync(TestContext.Current.CancellationToken);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.Is<AlertEvent>(a => a.Type == AlertType.DeploymentImageUpdateAvailable), It.IsAny<IEnumerable<Guid>>()), Times.Once);
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
        Assert.Equal(1, await GetAlertEventCountAsync(TestContext.Current.CancellationToken));
        await WaitForAlertStreamNotificationAsync(TestContext.Current.CancellationToken);

        await RunAutoUpdateJobOnceAsync(TestContext.Current.CancellationToken);

        await using var finalScope = Services.CreateAsyncScope();
        var db = finalScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Single(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>(), It.IsAny<IEnumerable<Guid>>()), Times.Once);
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

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>(), It.IsAny<IEnumerable<Guid>>()), Times.Never);
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

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>(), It.IsAny<IEnumerable<Guid>>()), Times.Never);
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

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(null, null, null, 1, 50, TestContext.Current.CancellationToken);

        Assert.Empty(alertEvents.Items);
        _alertEventStreamManager.Verify(x => x.SendTriggeredAlertEvent(It.IsAny<AlertEvent>(), It.IsAny<IEnumerable<Guid>>()), Times.Never);
    }

    private async Task RunAutoUpdateJobOnceAsync(CancellationToken cancellationToken)
    {
        var hostedServices = Services.GetServices<IHostedService>().ToArray();
        var scannerJob = hostedServices.OfType<DeploymentImageScannerJob>().Single();
        var autoUpdateJob = hostedServices.OfType<DeploymentAutoUpdateJob>().Single();
        var checkpoint = _dbWorkQueue.CreateCheckpoint();

        await scannerJob.ScanOnceAsync(cancellationToken);
        await autoUpdateJob.RunOnceAsync(cancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(checkpoint, cancellationToken);
    }

    [Fact]
    public async Task SwarmServiceOperationFailure_ShouldPersistServiceScopedAlert()
    {
        await EnsureAlertRuleCacheLoadedAsync(TestContext.Current.CancellationToken);
        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        var serviceId = Guid.CreateVersion7();
        var operationId = Guid.CreateVersion7();
        var alertService = Services.GetRequiredService<IAlertService>();

        await alertService.ProcessAsync(
            AlertType.SwarmServiceOperationFailed,
            new AlertEvaluationContext(
                DateTime.UtcNow,
                [],
                [],
                [],
                SwarmServiceOperationFailures:
                [
                    new SwarmServiceOperationFailureAlertSnapshot(
                        serviceId,
                        "redis",
                        operationId,
                        SwarmServiceOperationKind.Apply,
                        "rollout paused")
                ]),
            TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(checkpoint, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(
            null,
            null,
            null,
            1,
            50,
            TestContext.Current.CancellationToken);

        var alert = Assert.Single(alertEvents.Items);
        Assert.Equal(AlertType.SwarmServiceOperationFailed, alert.Type);
        Assert.Equal(AlertResourceType.SwarmService, alert.ResourceType);
        Assert.Equal(serviceId, alert.ResourceId);
        Assert.Equal(operationId, Assert.IsType<SwarmServiceOperationFailedAlertInfo>(alert.Info).OperationId);
        await WaitForAlertStreamNotificationAsync(TestContext.Current.CancellationToken);
    }

    private async Task<int> GetAlertEventCountAsync(CancellationToken cancellationToken)
    {
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var alertEvents = await db.AlertEvents.GetPagedAsync(
            null,
            null,
            null,
            1,
            50,
            cancellationToken);
        return alertEvents.Items.Count();
    }

    private async Task WaitForAlertStreamNotificationAsync(CancellationToken cancellationToken)
    {
        await _alertNotification.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            cancellationToken);
        _alertEventStreamManager.Verify(
            x => x.SendTriggeredAlertEvent(
                It.IsAny<AlertEvent>(),
                It.IsAny<IEnumerable<Guid>>()),
            Times.Once);
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
