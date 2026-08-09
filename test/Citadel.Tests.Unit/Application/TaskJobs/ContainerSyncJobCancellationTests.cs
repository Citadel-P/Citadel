using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using LightResults;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using System.Collections.Immutable;

namespace Tests.Unit.Application.TaskJobs;

public sealed class ContainerSyncJobCancellationTests
{
    [Fact]
    public async Task PeriodicBarrierWait_ReceivesHostedServiceCancellationToken()
    {
        var platform = new PlatformCacheEntry(
            Guid.CreateVersion7(),
            "unix:///var/run/docker.sock",
            PlatformConnectorType.Local,
            ImmutableDictionary<string, Guid>.Empty);
        var platforms = new[] { platform }.AsEnumerable();
        Error? cacheError = null;
        var cache = new Mock<IPlatformContainerCache>();
        cache
            .Setup(value => value.TryGetCacheEntries(
                out platforms,
                out cacheError))
            .Returns(true);
        var observed = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        var barrier = new Mock<ISyncBarrier>();
        barrier
            .Setup(value => value.WaitForAsync<ImageSyncJob>(
                platform.Id,
                It.IsAny<CancellationToken>()))
            .Callback((Guid _, CancellationToken token) => observed.TrySetResult(token.CanBeCanceled))
            .Returns(ValueTask.CompletedTask);
        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(value => value.ListContainersAsync(
                It.IsAny<ContainerFilterCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<IReadOnlyDictionary<string, DockerContainer>>("unavailable"));
        var connectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        connectorFactory
            .Setup(value => value.GetConnector(PlatformConnectorType.Local))
            .Returns(connector.Object);
        var broadcaster = new PlatformHealthBroadCaster();
        var scopeFactory = Mock.Of<IServiceScopeFactory>();
        var job = new ContainerSyncJob(
            Mock.Of<IDbWorkQueue>(),
            barrier.Object,
            Mock.Of<INotificationQueue>(),
            cache.Object,
            Mock.Of<IContainerStreamManager>(),
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IStackStreamManager>(),
            broadcaster,
            connectorFactory.Object,
            new SwarmTaskContainerPruner(
                scopeFactory,
                NullLogger<SwarmTaskContainerPruner>.Instance),
            NullLogger<ContainerSyncJob>.Instance);

        await job.StartAsync(TestContext.Current.CancellationToken);
        try
        {
            Assert.True(await observed.Task.WaitAsync(
                TimeSpan.FromSeconds(2),
                TestContext.Current.CancellationToken));
        }
        finally
        {
            await job.StopAsync(TestContext.Current.CancellationToken);
            broadcaster.Complete();
        }
    }
}
