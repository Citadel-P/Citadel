using System.Collections.Immutable;
using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Configuration;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Configuration;
using Domain.Entities.Deployments;
using Domain.Entities.Identity;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class ApplyDeploymentServiceTests
{
    [Fact]
    public async Task ApplyAsync_Should_Inject_And_Snapshot_Only_Configured_Citadel_Entries()
    {
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var deployment = new Deployment(
            name: "api",
            createdByActorId: actorId,
            platformId: platformId,
            spec: new DeploymentSpec(
                Image: new LocalImage("image-id"),
                UpdateBehavior: UpdateBehavior.Disabled,
                EnvironmentVariables:
                [
                    "APP_MODE",
                    "CONNECTION=${API_KEY}",
                    "STATIC=value"
                ]));

        ApplyDeploymentCommand? capturedCommand = null;
        ActivityEvent? capturedActivity = null;

        var deployments = new Mock<IDeploymentRepository>();
        deployments
            .Setup(x => x.GetAsync(deployment.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(deployment);
        deployments
            .Setup(x => x.UpdateAsync(deployment, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((activity, _) => capturedActivity = activity)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Deployments).Returns(deployments.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        var deploymentConnector = new Mock<IDeploymentConnector>();
        deploymentConnector
            .Setup(x => x.ApplyDeploymentAsync(It.IsAny<ApplyDeploymentCommand>(), It.IsAny<CancellationToken>()))
            .Callback<ApplyDeploymentCommand, CancellationToken>((command, _) => capturedCommand = command)
            .ReturnsAsync(Result.Failure<ApplyDeploymentResult>("failed with used-secret and orphan-token"));

        var deploymentConnectorFactory = new Mock<IConnectorFactory<IDeploymentConnector>>();
        deploymentConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(deploymentConnector.Object);

        var service = new ApplyDeploymentService(
            new InlineDbWorkQueue(unitOfWork.Object),
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IPullImageService>(),
            new ImageDigestCache(),
            new TestNotificationQueue(),
            new TestPlatformContainerCache(new PlatformCacheEntry(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty)),
            new StaticConfigurationResolver(new ResolvedConfiguration(
                EnvironmentVariables: ["APP_MODE=prod", "API_KEY=used-secret", "UNUSED_SECRET=orphan-token"],
                Entries:
                [
                    new ResolvedConfigurationEntry("APP_MODE", ConfigurationEntryKind.Variable, "prod"),
                    new ResolvedConfigurationEntry("API_KEY", ConfigurationEntryKind.Secret, "used-secret"),
                    new ResolvedConfigurationEntry("UNUSED_SECRET", ConfigurationEntryKind.Secret, "orphan-token")
                ],
                RedactionValues: ["used-secret", "orphan-token"],
                VariableCount: 1,
                SecretCount: 2)
            {
                SnapshotEntries =
                [
                    new ConfigurationSnapshotEntry(
                        Name: "APP_MODE",
                        Kind: ConfigurationEntryKind.Variable,
                        Scope: ConfigurationScope.Deployment,
                        ResourceId: deployment.Id,
                        Value: "prod",
                        SecretId: null,
                        SecretName: null,
                        SecretProviderType: null,
                        SecretProviderName: null,
                        ExternalPath: null,
                        ExternalKey: null,
                        ExternalVersion: null,
                        SecretDeliveryMode: null,
                        TargetPath: null),
                    new ConfigurationSnapshotEntry(
                        Name: "API_KEY",
                        Kind: ConfigurationEntryKind.Secret,
                        Scope: ConfigurationScope.Deployment,
                        ResourceId: deployment.Id,
                        Value: "********",
                        SecretId: Guid.CreateVersion7(),
                        SecretName: "api-key",
                        SecretProviderType: SecretProviderType.InternalEncrypted,
                        SecretProviderName: null,
                        ExternalPath: null,
                        ExternalKey: null,
                        ExternalVersion: null,
                        SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                        TargetPath: null),
                    new ConfigurationSnapshotEntry(
                        Name: "UNUSED_SECRET",
                        Kind: ConfigurationEntryKind.Secret,
                        Scope: ConfigurationScope.Deployment,
                        ResourceId: deployment.Id,
                        Value: "********",
                        SecretId: Guid.CreateVersion7(),
                        SecretName: "unused-secret",
                        SecretProviderType: SecretProviderType.InternalEncrypted,
                        SecretProviderName: null,
                        ExternalPath: null,
                        ExternalKey: null,
                        ExternalVersion: null,
                        SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                        TargetPath: null)
                ]
            }),
            new SecretRedactor(),
            Mock.Of<IAlertService>(),
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            deploymentConnectorFactory.Object);

        var items = new List<DeploymentStreamItem>();
        await foreach (var item in service.ApplyAsync(deployment.Id, actorId, recreate: false, TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.NotNull(capturedCommand);
        Assert.Equal(
            ["APP_MODE=prod", "CONNECTION=used-secret", "STATIC=value"],
            capturedCommand!.EnvironmentVariables);
        Assert.DoesNotContain(capturedCommand.EnvironmentVariables ?? [], value => value.Contains("orphan-token", StringComparison.Ordinal));

        var applied = Assert.IsType<DeploymentApplied>(capturedActivity?.Info);
        Assert.Equal("failed with ******** and orphan-token", applied.Result?.Message);
        Assert.Equal(["APP_MODE", "API_KEY"], applied.Result?.Configuration?.Select(entry => entry.Name));
    }

    private sealed class StaticConfigurationResolver(ResolvedConfiguration configuration) : IConfigurationResolver
    {
        public Task<Result<ResolvedConfiguration>> ResolveAsync(
            ConfigurationScope scope,
            Guid resourceId,
            CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(configuration));
    }

    private sealed class InlineDbWorkQueue(IUnitOfWork unitOfWork) : IDbWorkQueue
    {
        public ChannelReader<IDbWorkItem> Reader { get; } = Channel.CreateUnbounded<IDbWorkItem>().Reader;

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
            => EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await item.ExecuteAsync(unitOfWork, cancellationToken);
        }
    }

    private sealed class TestNotificationQueue : INotificationQueue
    {
        public ChannelReader<INotificationWorkItem> Reader { get; } = Channel.CreateUnbounded<INotificationWorkItem>().Reader;

        public ValueTask EnqueueAsync(INotificationWorkItem item, CancellationToken ct)
            => ValueTask.CompletedTask;
    }

    private sealed class TestPlatformContainerCache(PlatformCacheEntry platform) : IPlatformContainerCache
    {
        public void ReplacePlatformContainers(Guid platformId, PlatformCacheEntry cacheEntry)
        {
        }

        public bool TryAddContainer(Guid platformId, string containerId, Guid dbId) => false;

        public bool TryRemoveContainer(Guid platformId, string containerId) => false;

        public bool EvictPlatform(Guid platformId) => false;

        public bool TryGetContainers(Guid platformId, out IReadOnlyDictionary<string, Guid> containers)
        {
            containers = platform.Id == platformId
                ? platform.Containers
                : ImmutableDictionary<string, Guid>.Empty;
            return platform.Id == platformId;
        }

        public bool TryGetCacheEntry(Guid platformId, out PlatformCacheEntry cacheEntry, out Error error)
        {
            if (platform.Id == platformId)
            {
                cacheEntry = platform;
                error = null!;
                return true;
            }

            cacheEntry = null!;
            error = new Error("not found");
            return false;
        }

        public bool TryGetCacheEntries(out IEnumerable<PlatformCacheEntry> cacheEntries, out Error error)
        {
            cacheEntries = [platform];
            error = null!;
            return true;
        }

        public bool TryGetPlatformWithContainer(string containerId, out PlatformCacheEntry cacheEntry)
        {
            cacheEntry = null!;
            return false;
        }

        public bool TryGetPlatformsWithContainers(string[] containersId, out List<PlatformCacheEntry> cacheEntries)
        {
            cacheEntries = [];
            return false;
        }
    }
}
