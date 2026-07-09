using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class ContainerRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task UpdateContainersStateAsync_ShouldPersistEnumAsString()
    {
        var platform = new Platform(
            name: $"platform-{Guid.CreateVersion7():N}",
            address: $"edge://{Guid.CreateVersion7():D}",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1,
            serverVersion: null,
            agentVersion: null,
            status: PlatformStatus.Offline,
            connectorType: PlatformConnectorType.EdgeAgent,
            platformDescriptor: new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));

        var container = new Container(
            name: "container-1",
            dockerImageId: "image-1",
            platformId: platform.Id,
            ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            dockerContainerId: "container-1",
            state: ContainerStateStatus.Running);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Containers.UpdateContainersStateAsync(
                [container.Id],
                ContainerStateStatus.Offline,
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var updated = await uow.Containers.GetByIdAsync([container.Id], TestContext.Current.CancellationToken);
            Assert.Equal(ContainerStateStatus.Offline, Assert.Single(updated).State);
        }
    }
}
