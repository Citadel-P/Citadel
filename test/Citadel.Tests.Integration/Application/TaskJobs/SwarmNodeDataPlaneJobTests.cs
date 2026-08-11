using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public sealed class SwarmNodeDataPlaneJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task ReconciliationSnapshot_ShouldNotDeleteContainerObservedByNewerEvent()
    {
        var platform = Fakes.GetDummyPlatform();
        const string dockerNodeId = "worker-node";
        var snapshotStartedAt = DateTimeOffset.UtcNow.AddSeconds(-10).ToUnixTimeSeconds();
        var eventObservedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var container = new Container(
            "event-container",
            "image-id",
            platform.Id,
            "event-container-id",
            ContainerStateStatus.Running,
            dockerNodeId: dockerNodeId,
            projectionObservedAt: eventObservedAt);
        var state = new SwarmNodeRuntimeProjectionState(
            platform.Id,
            dockerNodeId,
            1,
            DateTimeOffset.FromUnixTimeSeconds(snapshotStartedAt),
            null,
            null,
            true,
            DateTimeOffset.UtcNow,
            "Initial reconciliation",
            null,
            null,
            null,
            null,
            null);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await uow.Swarm.UpsertNodeRuntimeStateAsync(state, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        using var cancellation = new CancellationTokenSource();
        using var monitor = new SwarmNodeDataPlaneJob.NodeMonitor(
            new SwarmNodeDataPlaneJob.NodeKey(platform.Id, dockerNodeId),
            "session",
            ownsStreams: true,
            cancellation);
        var workItem = new SwarmNodeDataPlaneJob.ReconcileSwarmNodeContainersWorkItem(
            monitor.Key,
            new Dictionary<string, DockerContainer>(),
            snapshotStartedAt,
            generation: 1,
            monitor,
            Mock.Of<INotificationQueue>(),
            new PlatformContainerCache(),
            Mock.Of<IContainerStreamManager>(),
            Mock.Of<IDockerDaemonStreamManager>());

        await workItem.ExecuteAsync(uow, TestContext.Current.CancellationToken);

        var persisted = await uow.Containers.GetByIdAsync(container.Id, TestContext.Current.CancellationToken);
        Assert.NotNull(persisted);
        Assert.Equal(eventObservedAt, persisted.ProjectionObservedAt);
        Assert.Equal(container.Id, monitor.ContainerIds[container.DockerContainerId]);
    }
}
