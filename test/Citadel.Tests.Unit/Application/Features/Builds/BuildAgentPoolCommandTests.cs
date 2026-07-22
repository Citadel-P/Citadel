using Application.Features.Builds.Commands;
using Application.Services;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using Hosting.Common.Abstraction;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Features.Builds;

public sealed class BuildAgentPoolCommandTests
{
    [Fact]
    public async Task TestBuildAgentPool_ShouldMarkReady_WhenSelfManagedAgentIsReachable()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var context = CreateContext(pool, actorId);
        context.ImageConnector
            .Setup(x => x.CheckBuildHostAsync("https://builder.internal:8443", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new BuildHostCapabilitiesResult(
                true,
                "28.0.0",
                "1.49",
                "linux",
                "amd64",
                "v0.20.2")));

        var result = await context.Handler.Handle(new TestBuildAgentPool(pool.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var tested, out var error), error?.Message);
        Assert.Equal(BuildAgentPoolValidationStatus.Ready, tested.Pool.LastValidationStatus);
        Assert.Equal("Docker 28.0.0 - API 1.49 - linux/amd64 - BuildKit v0.20.2", tested.Pool.LastValidationMessage);
        Assert.NotNull(tested.Pool.LastValidatedAt);
        Assert.Equal(ResourceControlState.Idle, tested.Pool.ControlState);
        context.BuildAgentPools.Verify(x => x.UpdateAsync(pool, It.IsAny<CancellationToken>()), Times.Once);
        context.BuildAgentPools.Verify(
            x => x.UpdateProcessingAsync(
                pool.Id,
                ResourceControlState.Processing,
                It.IsAny<long?>(),
                It.IsAny<long>(),
                true,
                actorId,
                It.IsAny<CancellationToken>()),
            Times.Once);
        context.ActivityEvents.Verify(
            x => x.AddAsync(
                It.Is<ActivityEvent>(activity => ActivityIsReadyBuildPoolTest(activity)),
                It.IsAny<CancellationToken>()),
            Times.Once);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Exactly(2));
        context.StreamManager.Verify(x => x.SendBuildAgentPoolInfo(pool, "update"), Times.Exactly(2));
        context.NotificationQueue.Verify(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task TestBuildAgentPool_ShouldMarkReady_WhenBuildKitVersionIsNotReported()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var context = CreateContext(pool, actorId);
        context.ImageConnector
            .Setup(x => x.CheckBuildHostAsync("https://builder.internal:8443", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new BuildHostCapabilitiesResult(
                true,
                "29.6.1",
                "1.55",
                "linux",
                "amd64",
                null)));

        var result = await context.Handler.Handle(new TestBuildAgentPool(pool.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var tested, out var error), error?.Message);
        Assert.Equal(BuildAgentPoolValidationStatus.Ready, tested.Pool.LastValidationStatus);
        Assert.Equal("Docker 29.6.1 - API 1.55 - linux/amd64", tested.Pool.LastValidationMessage);
    }

    [Fact]
    public async Task TestBuildAgentPool_ShouldMarkInvalid_WhenSelfManagedAgentIsUnreachable()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var context = CreateContext(pool, actorId);
        context.ImageConnector
            .Setup(x => x.CheckBuildHostAsync("https://builder.internal:8443", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<BuildHostCapabilitiesResult>(new Hosting.Common.ErrorTypes.InternalServerError("Docker ping failed")));

        var result = await context.Handler.Handle(new TestBuildAgentPool(pool.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var tested, out var error), error?.Message);
        Assert.Equal(BuildAgentPoolValidationStatus.Invalid, tested.Pool.LastValidationStatus);
        Assert.Equal("Citadel Agent build capability check failed: Docker ping failed", tested.Pool.LastValidationMessage);
        Assert.NotNull(tested.Pool.LastValidatedAt);
        context.BuildAgentPools.Verify(x => x.UpdateAsync(pool, It.IsAny<CancellationToken>()), Times.Once);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Exactly(2));
    }

    [Fact]
    public async Task TestBuildAgentPool_ShouldMarkInvalid_WhenHealthCheckThrows()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var context = CreateContext(pool, actorId);
        context.ImageConnector
            .Setup(x => x.CheckBuildHostAsync("https://builder.internal:8443", It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("connection refused"));

        var result = await context.Handler.Handle(new TestBuildAgentPool(pool.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var tested, out var error), error?.Message);
        Assert.Equal(BuildAgentPoolValidationStatus.Invalid, tested.Pool.LastValidationStatus);
        Assert.Equal("Citadel Agent build capability check failed: connection refused", tested.Pool.LastValidationMessage);
        context.BuildAgentPools.Verify(x => x.UpdateAsync(pool, It.IsAny<CancellationToken>()), Times.Once);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Exactly(2));
    }

    [Fact]
    public async Task TestBuildAgentPool_ShouldUseEdgeConnector_WhenSelfManagedPoolUsesEdgeMode()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(
            actorId,
            new SelfManagedVmBuildAgentPoolProviderSpec(
                null,
                CpuArchitecture.Amd64,
                1,
                ConnectionMode: BuildAgentPoolConnectionMode.EdgeAgent));
        var context = CreateContext(pool, actorId, connectorType: PlatformConnectorType.EdgeAgent);
        context.ImageConnector
            .Setup(x => x.CheckBuildHostAsync($"edge-build-pool://{pool.Id:D}", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new BuildHostCapabilitiesResult(
                true,
                "28.0.0",
                "1.49",
                "linux",
                "amd64",
                "v0.20.2")));

        var result = await context.Handler.Handle(new TestBuildAgentPool(pool.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var tested, out var error), error?.Message);
        Assert.Equal(BuildAgentPoolValidationStatus.Ready, tested.Pool.LastValidationStatus);
        context.ImageConnectorFactory.Verify(x => x.GetConnector(PlatformConnectorType.EdgeAgent), Times.Once);
    }

    [Fact]
    public async Task TestBuildAgentPool_ShouldRejectUnsupportedProvider()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateAwsPool(actorId);
        var context = CreateContext(pool, actorId, setupConnector: false);

        var result = await context.Handler.Handle(new TestBuildAgentPool(pool.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("Only self-managed", error.Message, StringComparison.OrdinalIgnoreCase);
        context.BuildAgentPools.Verify(x => x.UpdateAsync(It.IsAny<BuildAgentPool>(), It.IsAny<CancellationToken>()), Times.Never);
        context.BuildAgentPools.Verify(
            x => x.UpdateProcessingAsync(
                It.IsAny<Guid>(),
                It.IsAny<ResourceControlState>(),
                It.IsAny<long?>(),
                It.IsAny<long>(),
                It.IsAny<bool>(),
                It.IsAny<Guid?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        context.NotificationQueue.Verify(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    private static HandlerContext CreateContext(
        BuildAgentPool pool,
        Guid actorId,
        bool setupConnector = true,
        PlatformConnectorType connectorType = PlatformConnectorType.Agent)
    {
        var buildAgentPools = new Mock<IBuildAgentPoolRepository>(MockBehavior.Strict);
        buildAgentPools
            .Setup(x => x.GetAsync(pool.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(pool);
        buildAgentPools
            .Setup(x => x.UpdateAsync(pool, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        buildAgentPools
            .Setup(x => x.UpdateProcessingAsync(
                pool.Id,
                ResourceControlState.Processing,
                It.IsAny<long?>(),
                It.IsAny<long>(),
                true,
                actorId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var activityEvents = new Mock<IActivityEventRepository>(MockBehavior.Strict);
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>(MockBehavior.Strict);
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((global::Domain.Entities.Identity.Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.BuildAgentPools).Returns(buildAgentPools.Object);
        unitOfWork.SetupGet(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.SetupGet(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var imageConnector = new Mock<IImageConnector>(MockBehavior.Strict);
        var imageConnectorFactory = new Mock<IConnectorFactory<IImageConnector>>(MockBehavior.Strict);
        if (setupConnector)
        {
            imageConnectorFactory
                .Setup(x => x.GetConnector(connectorType))
                .Returns(imageConnector.Object);
        }

        var notificationQueue = new Mock<INotificationQueue>(MockBehavior.Strict);
        notificationQueue
            .Setup(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        var streamManager = new Mock<IBuildAgentPoolStreamManager>(MockBehavior.Strict);
        streamManager
            .Setup(x => x.SendBuildAgentPoolInfo(pool, "update"))
            .Returns(Task.CompletedTask);

        var handler = new TestBuildAgentPoolHandler(
            unitOfWork.Object,
            CreateUserContextAccessor(actorId),
            new BuildAgentPoolValidationService(imageConnectorFactory.Object),
            streamManager.Object,
            Mock.Of<IActivityStreamManager>(),
            notificationQueue.Object);

        return new HandlerContext(
            handler,
            unitOfWork,
            buildAgentPools,
            activityEvents,
            imageConnector,
            imageConnectorFactory,
            streamManager,
            notificationQueue);
    }

    private static BuildAgentPool CreateSelfManagedPool(
        Guid actorId,
        SelfManagedVmBuildAgentPoolProviderSpec? providerSpec = null)
        => new(
            "self managed builders",
            null,
            true,
            providerSpec ?? new SelfManagedVmBuildAgentPoolProviderSpec(
                "https://builder.internal:8443",
                CpuArchitecture.Amd64,
                1),
            BuildAgentPool.DefaultMaxActiveBuilders,
            BuildAgentPool.DefaultQueueTimeoutSeconds,
            BuildAgentPool.DefaultProvisioningTimeoutSeconds,
            BuildAgentPool.DefaultRegistrationTimeoutSeconds,
            BuildAgentPool.DefaultHeartbeatTimeoutSeconds,
            BuildAgentPool.DefaultCleanupTimeoutSeconds,
            BuildAgentPool.DefaultMaximumInstanceLifetimeSeconds,
            BuildAgentPool.DefaultFailureRetentionMinutes,
            actorId);

    private static BuildAgentPool CreateAwsPool(Guid actorId)
        => new(
            "aws builders",
            null,
            true,
            new AwsEc2BuildAgentPoolProviderSpec(
                "eu-west-1",
                "c5.2xlarge",
                CpuArchitecture.Amd64,
                "ami-123",
                80,
                "subnet-123",
                ["sg-123"],
                null,
                true,
                null,
                null,
                null),
            BuildAgentPool.DefaultMaxActiveBuilders,
            BuildAgentPool.DefaultQueueTimeoutSeconds,
            BuildAgentPool.DefaultProvisioningTimeoutSeconds,
            BuildAgentPool.DefaultRegistrationTimeoutSeconds,
            BuildAgentPool.DefaultHeartbeatTimeoutSeconds,
            BuildAgentPool.DefaultCleanupTimeoutSeconds,
            BuildAgentPool.DefaultMaximumInstanceLifetimeSeconds,
            BuildAgentPool.DefaultFailureRetentionMinutes,
            actorId);

    private static IUserContextAccessor CreateUserContextAccessor(Guid actorId)
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(x => x.UserId).Returns(Guid.CreateVersion7());
        user.SetupGet(x => x.ActorId).Returns(actorId);
        user.SetupGet(x => x.IsAdmin).Returns(true);
        user.SetupGet(x => x.IsAuthenticated).Returns(true);
        user.SetupGet(x => x.Roles).Returns(["admin"]);

        var accessor = new Mock<IUserContextAccessor>();
            accessor.SetupGet(x => x.Current).Returns(user.Object);
        return accessor.Object;
    }

    private static bool ActivityIsReadyBuildPoolTest(ActivityEvent activity)
        => activity.EventType == ActivityEventType.BuildAgentPoolTested &&
           activity.Info is BuildAgentPoolTested { Status: BuildAgentPoolValidationStatus.Ready };

    private sealed record HandlerContext(
        TestBuildAgentPoolHandler Handler,
        Mock<IUnitOfWork> UnitOfWork,
        Mock<IBuildAgentPoolRepository> BuildAgentPools,
        Mock<IActivityEventRepository> ActivityEvents,
        Mock<IImageConnector> ImageConnector,
        Mock<IConnectorFactory<IImageConnector>> ImageConnectorFactory,
        Mock<IBuildAgentPoolStreamManager> StreamManager,
        Mock<INotificationQueue> NotificationQueue);
}
