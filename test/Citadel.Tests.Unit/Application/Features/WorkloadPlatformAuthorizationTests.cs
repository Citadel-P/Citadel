using Application.Features.Deployments.Commands;
using Application.Features.Stacks.Commands;
using Application.Services;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.MergePatch;
using Moq;

namespace Tests.Unit.Application.Features;

public sealed class WorkloadPlatformAuthorizationTests
{
    [Fact]
    public async Task CreateDeployment_ShouldNotWrite_WhenPlatformIsNotAccessible()
    {
        var userId = Guid.CreateVersion7();
        var platform = CreatePlatform();
        var deployments = new Mock<IDeploymentRepository>();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        platforms
            .Setup(repository => repository.CanAccessAsync(userId, platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        var uow = CreateUnitOfWork(deployments.Object, platforms.Object, Mock.Of<IStackRepository>());
        var handler = new CreateDeploymentHandler(
            uow.Object,
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IActivityStreamManager>(),
            CreateUserContext(userId),
            Mock.Of<ILicenseEntitlementService>());

        var result = await handler.Handle(
            new CreateDeployment(
                "deployment",
                platform.Id,
                null,
                new DeploymentSpec(
                    Image: new ExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
                    UpdateBehavior: UpdateBehavior.Disabled)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("not accessible", error.Message, StringComparison.OrdinalIgnoreCase);
        deployments.Verify(
            repository => repository.ExistsAsync(
                It.IsAny<string>(),
                It.IsAny<Guid>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        deployments.Verify(
            repository => repository.AddAsync(It.IsAny<Deployment>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task CreateStack_ShouldNotWrite_WhenPlatformIsNotAccessible()
    {
        var userId = Guid.CreateVersion7();
        var platform = CreatePlatform();
        var stacks = new Mock<IStackRepository>();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        platforms
            .Setup(repository => repository.CanAccessAsync(userId, platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        var uow = CreateUnitOfWork(Mock.Of<IDeploymentRepository>(), platforms.Object, stacks.Object);
        var handler = new CreateStackHandler(
            uow.Object,
            Mock.Of<IPlatformStreamManager>(),
            CreateUserContext(userId),
            Mock.Of<ILicenseEntitlementService>());

        var result = await handler.Handle(
            new CreateStack(
                "stack",
                platform.Id,
                null,
                StackSource.WebEditor,
                new ManualStack("services: {}", StackUpdateBehavior.Disabled)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("not accessible", error.Message, StringComparison.OrdinalIgnoreCase);
        stacks.Verify(
            repository => repository.ExistsAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()),
            Times.Never);
        stacks.Verify(
            repository => repository.AddAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task PatchDeployment_ShouldNotWrite_WhenPlatformIsNotAccessible()
    {
        var userId = Guid.CreateVersion7();
        var platform = CreatePlatform();
        var deployment = new Deployment(
            "deployment",
            userId,
            platform.Id,
            new DeploymentSpec(
                Image: new ExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
                UpdateBehavior: UpdateBehavior.Disabled));
        var deployments = new Mock<IDeploymentRepository>();
        deployments
            .Setup(repository => repository.GetAsync(deployment.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(deployment);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        platforms
            .Setup(repository => repository.CanAccessAsync(userId, platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        var uow = CreateUnitOfWork(deployments.Object, platforms.Object, Mock.Of<IStackRepository>());
        var handler = new PatchDeploymentHandler(
            uow.Object,
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IActivityStreamManager>(),
            CreateUserContext(userId),
            Mock.Of<ILicenseEntitlementService>());

        var result = await handler.Handle(
            new PatchDeployment(
                deployment.Id,
                JsonMergePatchDocument<Deployment>.FromJson("{}")),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("not accessible", error.Message, StringComparison.OrdinalIgnoreCase);
        deployments.Verify(
            repository => repository.UpdateAsync(It.IsAny<Deployment>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task PatchStack_ShouldNotWrite_WhenPlatformIsNotAccessible()
    {
        var userId = Guid.CreateVersion7();
        var platform = CreatePlatform();
        var stack = Stack.Create(
            "stack",
            userId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services: {}", StackUpdateBehavior.Disabled));
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(repository => repository.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        platforms
            .Setup(repository => repository.CanAccessAsync(userId, platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        var uow = CreateUnitOfWork(Mock.Of<IDeploymentRepository>(), platforms.Object, stacks.Object);
        var handler = new PatchStackHandler(
            uow.Object,
            Mock.Of<IPlatformStreamManager>(),
            CreateUserContext(userId),
            Mock.Of<ILicenseEntitlementService>());

        var result = await handler.Handle(
            new PatchStack(
                stack.Id,
                JsonMergePatchDocument<StackPatchModel>.FromJson("{}")),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("not accessible", error.Message, StringComparison.OrdinalIgnoreCase);
        stacks.Verify(
            repository => repository.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ApplyDeployment_ShouldNotCallService_WhenPlatformIsNotAccessible()
    {
        var userId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var deployment = new Deployment(
            name: "deployment",
            createdByActorId: userId,
            platformId: platformId,
            spec: new DeploymentSpec(
                Image: new ExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
                UpdateBehavior: UpdateBehavior.Disabled));
        var deployments = new Mock<IDeploymentRepository>();
        deployments
            .Setup(repository => repository.GetAsync(deployment.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(deployment);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.CanAccessAsync(userId, platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        var uow = CreateUnitOfWork(deployments.Object, platforms.Object, Mock.Of<IStackRepository>());
        var service = new Mock<IApplyDeploymentService>();
        var handler = new ApplyDeploymentHandler(service.Object, uow.Object, CreateUserContext(userId));

        var items = await ReadAllAsync(handler.Handle(
            new ApplyDeployment(deployment.Id),
            TestContext.Current.CancellationToken));

        var item = Assert.Single(items);
        Assert.Equal(404, item.Error?.Code);
        service.Verify(
            value => value.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<bool>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task ApplyStack_ShouldNotCallService_WhenPlatformIsNotAccessible()
    {
        var userId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var stack = Stack.Create(
            "stack",
            userId,
            StackSource.WebEditor,
            platformId,
            new ManualStack("services: {}", StackUpdateBehavior.Disabled));
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(repository => repository.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.CanAccessAsync(userId, platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        var uow = CreateUnitOfWork(Mock.Of<IDeploymentRepository>(), platforms.Object, stacks.Object);
        var service = new Mock<IApplyStackService>();
        var handler = new ApplyStackHandler(service.Object, uow.Object, CreateUserContext(userId));

        var items = await ReadAllAsync(handler.Handle(
            new ApplyStack(stack.Id),
            TestContext.Current.CancellationToken));

        var item = Assert.Single(items);
        Assert.Contains("not accessible", item.Message, StringComparison.OrdinalIgnoreCase);
        service.Verify(
            value => value.ApplyAsync(
                It.IsAny<Guid>(),
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyList<string>?>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<bool>(),
                It.IsAny<StackApplyOperation>(),
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        IDeploymentRepository deployments,
        IPlatformRepository platforms,
        IStackRepository stacks)
    {
        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(value => value.Deployments).Returns(deployments);
        uow.SetupGet(value => value.Platforms).Returns(platforms);
        uow.SetupGet(value => value.Stacks).Returns(stacks);
        return uow;
    }

    private static IUserContextAccessor CreateUserContext(Guid userId)
    {
        var accessor = new Mock<IUserContextAccessor>();
        accessor.SetupGet(value => value.Current).Returns(Mock.Of<IUserContext>(user =>
            user.UserId == userId
            && user.ActorId == userId
            && user.IsAuthenticated
            && !user.IsAdmin));
        return accessor.Object;
    }

    private static Platform CreatePlatform()
        => new(
            name: "platform",
            address: "https://docker.example.test",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "daemon-1",
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0));

    private static async Task<List<T>> ReadAllAsync<T>(IAsyncEnumerable<T> source)
    {
        var items = new List<T>();
        await foreach (var item in source.WithCancellation(TestContext.Current.CancellationToken))
            items.Add(item);
        return items;
    }
}
