using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class ProcessingClaimRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task ProcessingClaims_ShouldNotBeReclaimedByANewerCommand()
    {
        var ct = TestContext.Current.CancellationToken;
        var firstActor = Constants.SystemId;
        var secondActor = Guid.CreateVersion7();
        var platform = CreatePlatform();
        var deployment = new Deployment(
            $"deployment-{Guid.CreateVersion7():N}",
            Constants.SystemId,
            platform.Id);
        var stack = Stack.Create(
            $"stack-{Guid.CreateVersion7():N}",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services:\n  app:\n    image: nginx\n", StackUpdateBehavior.Disabled));
        var container = new Container(
            "container",
            "sha256:nginx",
            platform.Id,
            dockerContainerId: $"container-{Guid.CreateVersion7():N}",
            state: ContainerStateStatus.Exited);
        var image = new Image(
            "nginx",
            ["nginx:latest"],
            $"sha256:{Guid.CreateVersion7():N}{Guid.CreateVersion7():N}",
            1024,
            0,
            platform.Id,
            DateTime.UtcNow);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, ct);
            await uow.Deployments.AddAsync(deployment, ct);
            await uow.Stacks.AddAsync(stack, ct);
            await uow.Containers.AddAsync(container, ct);
            await uow.Images.AddOrUpdateAsync(image, ct);
            await uow.CommitAsync(ct);
        }

        var startedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var storedContainer = Assert.Single(await uow.Containers.GetByIdAsync([container.Id], ct));
            var storedDeployment = (await uow.Deployments.GetAsync(deployment.Id, ct))!;
            var storedStack = (await uow.Stacks.GetAsync(stack.Id, ct))!;
            var storedImage = (await uow.Images.GetByDockerImageIdAsync(image.DockerImageId, platform.Id, ct))!;

            Assert.Equal(1, await uow.Containers.UpdateProcessingAsync(
                storedContainer.Id, ResourceControlState.Processing, startedAt,
                storedContainer.RowVersion, true, firstActor, ct));
            Assert.Equal(1, await uow.Deployments.UpdateProcessingAsync(
                storedDeployment.Id, DeploymentStatus.Pending, ResourceControlState.Processing, startedAt,
                storedDeployment.RowVersion, true, firstActor, ct));
            Assert.True(await uow.Stacks.UpdateProcessingAsync(
                storedStack.Id, StackReleaseStatus.Pending, ResourceControlState.Processing, startedAt,
                storedStack.RowVersion, true, firstActor, ct));
            Assert.Equal(1, await uow.Images.UpdateProcessingAsync(
                storedImage.Id, ResourceControlState.Processing, startedAt,
                storedImage.RowVersion, true, ct));
            await uow.CommitAsync(ct);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var storedContainer = Assert.Single(await uow.Containers.GetByIdAsync([container.Id], ct));
            var storedDeployment = (await uow.Deployments.GetAsync(deployment.Id, ct))!;
            var storedStack = (await uow.Stacks.GetAsync(stack.Id, ct))!;
            var storedImage = (await uow.Images.GetByDockerImageIdAsync(image.DockerImageId, platform.Id, ct))!;

            Assert.Equal(0, await uow.Containers.UpdateProcessingAsync(
                storedContainer.Id, ResourceControlState.Processing, startedAt + 1,
                storedContainer.RowVersion, true, secondActor, ct));
            Assert.Equal(0, await uow.Deployments.UpdateProcessingAsync(
                storedDeployment.Id, DeploymentStatus.Pending, ResourceControlState.Processing, startedAt + 1,
                storedDeployment.RowVersion, true, secondActor, ct));
            Assert.False(await uow.Stacks.UpdateProcessingAsync(
                storedStack.Id, StackReleaseStatus.Pending, ResourceControlState.Processing, startedAt + 1,
                storedStack.RowVersion, true, secondActor, ct));
            Assert.Equal(0, await uow.Images.UpdateProcessingAsync(
                storedImage.Id, ResourceControlState.Processing, startedAt + 1,
                storedImage.RowVersion, true, ct));
            await uow.CommitAsync(ct);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var storedContainer = Assert.Single(await uow.Containers.GetByIdAsync([container.Id], ct));
            var storedDeployment = (await uow.Deployments.GetAsync(deployment.Id, ct))!;
            var storedStack = (await uow.Stacks.GetAsync(stack.Id, ct))!;
            var storedImage = (await uow.Images.GetByDockerImageIdAsync(image.DockerImageId, platform.Id, ct))!;

            Assert.Equal(firstActor, storedContainer.ControlTriggeredBy);
            Assert.Equal(firstActor, storedDeployment.ControlTriggeredBy);
            Assert.Equal(firstActor, storedStack.ControlTriggeredBy);
            Assert.Equal(startedAt, storedContainer.ControlStartedAt);
            Assert.Equal(startedAt, storedDeployment.ControlStartedAt);
            Assert.Equal(startedAt, storedStack.ControlStartedAt);
            Assert.Equal(startedAt, storedImage.ControlStartedAt);
        }
    }

    private static Platform CreatePlatform()
        => new(
            $"platform-{Guid.CreateVersion7():N}",
            $"edge://{Guid.CreateVersion7():D}",
            0,
            0,
            0,
            1,
            1,
            null,
            null,
            PlatformStatus.Online,
            PlatformConnectorType.EdgeAgent,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
}
