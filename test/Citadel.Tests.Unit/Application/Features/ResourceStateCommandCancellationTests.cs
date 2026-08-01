using System.Collections.Immutable;
using Application.Features.Containers.Commands;
using Application.Features.Deployments.Commands;
using Application.Features.Stacks.Commands;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Features;

public sealed class ResourceStateCommandCancellationTests
{
    [Fact]
    public async Task PatchContainer_AfterClaim_ShouldFinishWithTokenIndependentOfRequest()
    {
        using var requestCancellation = new CancellationTokenSource();
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var container = CreateContainer(platformId);
        var resources = new ProcessedResources([container], [], []);
        var processing = new Mock<IContainerProcessingService>();
        processing
            .Setup(service => service.MarkProcessingAsync(
                It.IsAny<Guid[]>(),
                actorId,
                requestCancellation.Token,
                true))
            .Callback(requestCancellation.Cancel)
            .ReturnsAsync(resources);
        processing
            .Setup(service => service.NotifyProcessingAsync(resources, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        processing
            .Setup(service => service.CompleteProcessingAsync(
                resources,
                It.IsAny<IReadOnlyCollection<PlatformCacheEntry>>(),
                actorId))
            .Returns(Task.CompletedTask);
        var connector = CreateSuccessfulConnector();
        var platform = CreatePlatform(platformId, container);
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.GetByIdsAsync(
                It.IsAny<string[]>(),
                requestCancellation.Token))
            .ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Containers).Returns(containers.Object);
        var authorization = new Mock<IContainerAuthorizationService>();
        authorization
            .Setup(service => service.HasAccessAsync(
                It.IsAny<IEnumerable<string>>(),
                ResourceType.Platform,
                PermissionLevel.Write,
                SpecificPermission.None,
                requestCancellation.Token))
            .ReturnsAsync(true);
        var handler = new PatchContainerHandler(
            CreateUserContext(actorId),
            unitOfWork.Object,
            processing.Object,
            CreatePlatformCache(platform),
            CreateConnectorFactory(connector.Object),
            authorization.Object,
            CreateApplicationLifetime(),
            Mock.Of<ILogger<PatchContainerHandler>>());

        var result = await handler.Handle(
            new PatchContainer([container.DockerContainerId], ContainerAction.START),
            requestCancellation.Token);

        Assert.True(result.IsSuccess());
        connector.Verify(service => service.PatchAsync(
            It.IsAny<PatchContainerCommand>(),
            It.Is<CancellationToken>(token => !token.IsCancellationRequested)), Times.Once);
    }

    [Fact]
    public async Task ChangeDeploymentState_AfterClaim_ShouldFinishWithTokenIndependentOfRequest()
    {
        using var requestCancellation = new CancellationTokenSource();
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var deployment = new Deployment("deployment", actorId, platformId);
        var container = CreateContainer(platformId, deploymentId: deployment.Id);
        deployment.PartialUpdate(container: container);
        var processing = new Mock<IDeploymentProcessingService>();
        processing
            .Setup(service => service.MarkProcessingAsync(
                It.IsAny<IEnumerable<Guid>>(),
                actorId,
                requestCancellation.Token))
            .Callback(requestCancellation.Cancel)
            .ReturnsAsync([deployment]);
        processing
            .Setup(service => service.NotifyProcessingAsync(
                It.IsAny<IEnumerable<Deployment>>(),
                "update",
                It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        var containerProcessing = new Mock<IContainerProcessingService>();
        containerProcessing
            .Setup(service => service.CompleteProcessingAsync(
                It.IsAny<ProcessedResources>(),
                It.IsAny<IReadOnlyCollection<PlatformCacheEntry>>(),
                actorId))
            .Returns(Task.CompletedTask);
        var connector = CreateSuccessfulConnector();
        var platform = CreatePlatform(platformId, container);
        var handler = new ChangeDeploymentStateHandler(
            processing.Object,
            containerProcessing.Object,
            CreatePlatformCache(platform),
            CreateUserContext(actorId),
            CreateConnectorFactory(connector.Object),
            CreateApplicationLifetime(),
            Mock.Of<ILogger<ChangeDeploymentStateHandler>>());

        var result = await handler.Handle(
            new ChangeDeploymentState([deployment.Id], DeploymentAction.START),
            requestCancellation.Token);

        Assert.True(result.IsSuccess());
        connector.Verify(service => service.PatchAsync(
            It.IsAny<PatchContainerCommand>(),
            It.Is<CancellationToken>(token => !token.IsCancellationRequested)), Times.Once);
    }

    [Fact]
    public async Task ChangeStackState_AfterClaim_ShouldFinishWithTokenIndependentOfRequest()
    {
        using var requestCancellation = new CancellationTokenSource();
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var stack = Stack.Create(
            "stack",
            actorId,
            StackSource.WebEditor,
            platformId,
            new ManualStack("services:\n  web:\n    image: nginx", StackUpdateBehavior.Disabled));
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        var container = CreateContainer(platformId, stackId: stack.Id);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(repository => repository.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        stacks
            .Setup(repository => repository.UpdateProcessingAsync(
                stack.Id,
                It.IsAny<StackReleaseStatus>(),
                It.IsAny<ResourceControlState>(),
                It.IsAny<long?>(),
                It.IsAny<long>(),
                true,
                actorId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        stacks
            .Setup(repository => repository.GetContainersAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([container]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Stacks).Returns(stacks.Object);
        unitOfWork
            .Setup(work => work.CommitAsync(requestCancellation.Token))
            .Callback(requestCancellation.Cancel)
            .Returns(Task.CompletedTask);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var connector = CreateSuccessfulConnector();
        var platform = CreatePlatform(platformId, container);
        var containerProcessing = new Mock<IContainerProcessingService>();
        containerProcessing
            .Setup(service => service.CompleteProcessingAsync(
                It.IsAny<ProcessedResources>(),
                It.IsAny<IReadOnlyCollection<PlatformCacheEntry>>(),
                actorId))
            .Returns(Task.CompletedTask);
        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        var handler = new ChangeStackStateHandler(
            services.GetRequiredService<IServiceScopeFactory>(),
            CreateUserContext(actorId),
            notificationQueue.Object,
            Mock.Of<IStackStreamManager>(),
            CreatePlatformCache(platform),
            containerProcessing.Object,
            CreateConnectorFactory(connector.Object),
            CreateApplicationLifetime(),
            Mock.Of<ILogger<ChangeStackStateHandler>>());

        var result = await handler.Handle(
            new ChangeStackState([stack.Id], StackAction.START),
            requestCancellation.Token);

        Assert.True(result.IsSuccess());
        connector.Verify(service => service.PatchAsync(
            It.IsAny<PatchContainerCommand>(),
            It.Is<CancellationToken>(token => !token.IsCancellationRequested)), Times.Once);
    }

    [Fact]
    public async Task ChangeStackState_WhenRollbackClaimWasLost_ShouldNotPublishStaleIdleState()
    {
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var stack = Stack.Create(
            "stack",
            actorId,
            StackSource.WebEditor,
            platformId,
            new ManualStack("services:\n  web:\n    image: nginx", StackUpdateBehavior.Disabled));
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        var container = CreateContainer(platformId, stackId: stack.Id);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(repository => repository.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        stacks
            .Setup(repository => repository.UpdateProcessingAsync(
                stack.Id,
                It.IsAny<StackReleaseStatus>(),
                ResourceControlState.Processing,
                It.IsAny<long?>(),
                stack.RowVersion,
                true,
                actorId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        stacks
            .Setup(repository => repository.UpdateProcessingAsync(
                stack.Id,
                It.IsAny<StackReleaseStatus>(),
                ResourceControlState.Idle,
                null,
                stack.RowVersion + 1,
                true,
                null,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        stacks
            .Setup(repository => repository.GetContainersAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([container]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(work => work.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(service => service.PatchAsync(
                It.IsAny<PatchContainerCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure("connector unavailable"));
        var notificationQueue = new Mock<INotificationQueue>();
        notificationQueue
            .Setup(queue => queue.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        var handler = new ChangeStackStateHandler(
            services.GetRequiredService<IServiceScopeFactory>(),
            CreateUserContext(actorId),
            notificationQueue.Object,
            Mock.Of<IStackStreamManager>(),
            CreatePlatformCache(CreatePlatform(platformId, container)),
            Mock.Of<IContainerProcessingService>(),
            CreateConnectorFactory(connector.Object),
            CreateApplicationLifetime(),
            Mock.Of<ILogger<ChangeStackStateHandler>>());

        var result = await handler.Handle(
            new ChangeStackState([stack.Id], StackAction.START),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
        notificationQueue.Verify(
            queue => queue.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private static Container CreateContainer(Guid platformId, Guid? deploymentId = null, Guid? stackId = null)
        => new(
            "web",
            "sha256:image",
            platformId,
            "0123456789abcdef",
            ContainerStateStatus.Exited,
            deploymentId: deploymentId,
            stackId: stackId);

    private static PlatformCacheEntry CreatePlatform(Guid platformId, Container container)
        => new(
            platformId,
            "local://docker",
            PlatformConnectorType.Local,
            ImmutableDictionary<string, Guid>.Empty.Add(container.DockerContainerId, container.Id));

    private static IPlatformContainerCache CreatePlatformCache(PlatformCacheEntry platform)
    {
        var entries = new List<PlatformCacheEntry> { platform };
        var cache = new Mock<IPlatformContainerCache>();
        cache
            .Setup(value => value.TryGetPlatformsWithContainers(It.IsAny<string[]>(), out entries))
            .Returns(true);
        return cache.Object;
    }

    private static Mock<IContainerConnector> CreateSuccessfulConnector()
    {
        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(service => service.PatchAsync(
                It.IsAny<PatchContainerCommand>(),
                It.Is<CancellationToken>(token => !token.IsCancellationRequested)))
            .ReturnsAsync(Result.Success());
        return connector;
    }

    private static IConnectorFactory<IContainerConnector> CreateConnectorFactory(IContainerConnector connector)
    {
        var factory = new Mock<IConnectorFactory<IContainerConnector>>();
        factory.Setup(value => value.GetConnector(PlatformConnectorType.Local)).Returns(connector);
        return factory.Object;
    }

    private static IUserContextAccessor CreateUserContext(Guid actorId)
    {
        var context = new Mock<IUserContextAccessor>();
        context.SetupGet(value => value.Current).Returns(Mock.Of<IUserContext>(user => user.ActorId == actorId));
        return context.Object;
    }

    private static IHostApplicationLifetime CreateApplicationLifetime()
    {
        var lifetime = new Mock<IHostApplicationLifetime>();
        lifetime.SetupGet(value => value.ApplicationStopping).Returns(CancellationToken.None);
        return lifetime.Object;
    }
}
