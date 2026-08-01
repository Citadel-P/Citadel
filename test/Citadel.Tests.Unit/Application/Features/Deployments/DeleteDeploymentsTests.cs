using Application.Features.Deployments.Commands;
using Application.Features.Containers.Commands;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Features.Deployments;

public sealed class DeleteDeploymentsTests
{
    [Fact]
    public async Task Handle_ShouldRollbackAndKeepDeploymentWhenContainerDeletionFails()
    {
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var deployment = new Deployment(
            "deployment",
            actorId,
            platformId);
        var container = new Container(
            "container",
            "sha256:image",
            platformId,
            "docker-container-id",
            ContainerStateStatus.Running,
            deploymentId: deployment.Id);
        deployment.PartialUpdate(container: container);
        var processing = new Mock<IDeploymentProcessingService>();
        processing
            .Setup(service => service.MarkProcessingAsync(
                It.IsAny<IEnumerable<Guid>>(),
                actorId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([deployment]);
        var containers = new Mock<IContainerProcessingService>();
        containers
            .Setup(service => service.DeleteContainers(
                It.IsAny<DeleteContainers>(),
                actorId,
                It.IsAny<CancellationToken>(),
                false))
            .ReturnsAsync(Result.Failure("connector unavailable"));
        var user = new Mock<IUserContext>();
        user.SetupGet(value => value.ActorId).Returns(actorId);
        var userContext = new Mock<IUserContextAccessor>();
        userContext.SetupGet(value => value.Current).Returns(user.Object);
        var handler = new DeleteDeploymentsHandler(
            Mock.Of<IServiceScopeFactory>(),
            processing.Object,
            containers.Object,
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<INotificationQueue>(),
            userContext.Object,
            CreateApplicationLifetime(),
            Mock.Of<ILogger<DeleteDeploymentsHandler>>());

        var result = await handler.Handle(
            new DeleteDeployments([deployment.Id]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("connector unavailable", error.Message);
        processing.Verify(
            service => service.RollbackProcessingAsync(
                It.Is<IEnumerable<Deployment>>(values => values.Single() == deployment),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task Handle_ShouldFinishRollbackAfterRequestCancellationOnceClaimed()
    {
        using var requestCancellation = new CancellationTokenSource();
        var actorId = Guid.CreateVersion7();
        var deployment = new Deployment("deployment", actorId, Guid.CreateVersion7());
        var container = new Container(
            "container",
            "sha256:image",
            deployment.PlatformId,
            "docker-container-id",
            ContainerStateStatus.Running,
            deploymentId: deployment.Id);
        deployment.PartialUpdate(container: container);

        var processing = new Mock<IDeploymentProcessingService>();
        processing
            .Setup(service => service.MarkProcessingAsync(
                It.IsAny<IEnumerable<Guid>>(),
                actorId,
                requestCancellation.Token))
            .ReturnsAsync([deployment]);
        processing
            .Setup(service => service.NotifyProcessingAsync(
                It.IsAny<IEnumerable<Deployment>>(),
                "update",
                It.IsAny<CancellationToken>()))
            .Callback(() => requestCancellation.Cancel())
            .Returns(Task.CompletedTask);

        var containers = new Mock<IContainerProcessingService>();
        containers
            .Setup(service => service.DeleteContainers(
                It.IsAny<DeleteContainers>(),
                actorId,
                It.Is<CancellationToken>(token => !token.IsCancellationRequested),
                false))
            .ReturnsAsync(Result.Failure("connector unavailable"));

        var user = new Mock<IUserContext>();
        user.SetupGet(value => value.ActorId).Returns(actorId);
        var userContext = new Mock<IUserContextAccessor>();
        userContext.SetupGet(value => value.Current).Returns(user.Object);
        var handler = new DeleteDeploymentsHandler(
            Mock.Of<IServiceScopeFactory>(),
            processing.Object,
            containers.Object,
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<INotificationQueue>(),
            userContext.Object,
            CreateApplicationLifetime(),
            Mock.Of<ILogger<DeleteDeploymentsHandler>>());

        var result = await handler.Handle(
            new DeleteDeployments([deployment.Id]),
            requestCancellation.Token);

        Assert.True(result.IsFailure());
        processing.Verify(service => service.RollbackProcessingAsync(
            It.IsAny<IEnumerable<Deployment>>(),
            It.Is<CancellationToken>(token => !token.IsCancellationRequested)), Times.Once);
    }

    private static IHostApplicationLifetime CreateApplicationLifetime()
    {
        var lifetime = new Mock<IHostApplicationLifetime>();
        lifetime.SetupGet(value => value.ApplicationStopping).Returns(CancellationToken.None);
        return lifetime.Object;
    }
}
