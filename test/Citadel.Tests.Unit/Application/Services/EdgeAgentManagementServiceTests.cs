using Application.Configs;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class EdgeAgentManagementServiceTests
{
    [Fact]
    public void EdgeAgentOptions_ShouldUseCorePackageVersion_WhenTagIsNotConfigured()
    {
        var options = new EdgeAgentOptions();

        Assert.Equal($"ghcr.io/citadel-p/citadel.agent:{Constants.CompatibilityVersion}", options.GetAgentImage());
    }

    [Fact]
    public async Task CreateEnrollmentAsync_ShouldReturnServerGeneratedDockerCommand()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var platform = Platform.FromPersistence(
            id: platformId,
            name: "edge-platform",
            address: $"edge://{platformId:D}",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 0,
            memTotal: 0,
            status: PlatformStatus.Offline,
            connectorType: PlatformConnectorType.EdgeAgent,
            platformDescriptor: new DockerPlatformDescriptor("edge", 0, 0, 0, 0));
        EdgeAgentEnrollment? capturedEnrollment = null;

        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        platforms
            .Setup(x => x.GetByIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.GetBindingByResourceAsync(
                EdgeAgentResourceType.Platform,
                platformId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((EdgeAgentBinding?)null);
        edgeAgents
            .Setup(x => x.AddEnrollmentAsync(It.IsAny<EdgeAgentEnrollment>(), It.IsAny<CancellationToken>()))
            .Callback<EdgeAgentEnrollment, CancellationToken>((enrollment, _) => capturedEnrollment = enrollment)
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        unitOfWork
            .Setup(x => x.DisposeAsync())
            .Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var service = new EdgeAgentManagementService(
            provider.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Options.Create(new EdgeAgentOptions
            {
                AgentImageRepository = "registry.example.com/citadel-agent",
                AgentImageTag = "v1.2.3+build.9"
            }));

        var result = await service.CreateEnrollmentAsync(
            platformId,
            "https://citadel.example.com",
            actorId,
            TimeSpan.FromHours(1),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var enrollment, out var error), error?.Message);
        Assert.NotNull(capturedEnrollment);
        Assert.Equal(capturedEnrollment.Id, enrollment.EnrollmentId);
        Assert.Equal(platformId, enrollment.PlatformId);
        Assert.Equal(actorId, capturedEnrollment.CreatedByActorId);
        Assert.Equal(EdgeAgentManagementService.HashToken(enrollment.Token), capturedEnrollment.TokenHash);
        Assert.Equal("registry.example.com/citadel-agent:1.2.3", enrollment.Instructions.AgentImage);
        Assert.Contains("registry.example.com/citadel-agent:1.2.3", enrollment.Instructions.DockerRunCommand);
        Assert.Contains("CITADEL_CORE_URL=\"https://citadel.example.com\"", enrollment.Instructions.DockerRunCommand);
        Assert.Contains("CITADEL_EDGE_ENROLLMENT_TOKEN=", enrollment.Instructions.DockerRunCommand);

        platforms.VerifyAll();
        edgeAgents.VerifyAll();
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task MarkHeartbeatAsync_ShouldUpdateBindingAndCommit()
    {
        var platformId = Guid.CreateVersion7();
        var utcNow = new DateTime(2026, 7, 9, 18, 30, 0, DateTimeKind.Utc);
        var heartbeat = new EdgeAgentHeartbeatSnapshot(
            DockerReachable: true,
            DockerVersion: "27.5.1",
            Hostname: "edge-host",
            AgentVersion: "edge-agent-test",
            CapabilitiesJson: """{"containers":true,"logs":true}""");

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.UpdateBindingHeartbeatAsync(
                EdgeAgentResourceType.Platform,
                platformId,
                utcNow,
                heartbeat.Hostname,
                heartbeat.AgentVersion,
                heartbeat.CapabilitiesJson,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        unitOfWork
            .Setup(x => x.DisposeAsync())
            .Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var service = new EdgeAgentManagementService(
            provider.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Options.Create(new EdgeAgentOptions()));

        await service.MarkHeartbeatAsync(platformId, heartbeat, utcNow, TestContext.Current.CancellationToken);

        edgeAgents.VerifyAll();
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task CompleteEnrollmentAsync_ShouldRejectBuildPoolAgentWithoutBuildCapabilities()
    {
        var poolId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var token = "edge-token";
        var utcNow = new DateTime(2026, 7, 22, 8, 0, 0, DateTimeKind.Utc);
        var pool = new BuildAgentPool(
            "edge-builders",
            description: null,
            enabled: true,
            providerSpec: new SelfManagedVmBuildAgentPoolProviderSpec(
                Endpoint: null,
                Architecture: CpuArchitecture.Amd64,
                MaxWorkers: 1,
                ConnectionMode: BuildAgentPoolConnectionMode.EdgeAgent),
            maxActiveBuilders: 1,
            queueTimeoutSeconds: BuildAgentPool.DefaultQueueTimeoutSeconds,
            provisioningTimeoutSeconds: BuildAgentPool.DefaultProvisioningTimeoutSeconds,
            registrationTimeoutSeconds: BuildAgentPool.DefaultRegistrationTimeoutSeconds,
            heartbeatTimeoutSeconds: BuildAgentPool.DefaultHeartbeatTimeoutSeconds,
            cleanupTimeoutSeconds: BuildAgentPool.DefaultCleanupTimeoutSeconds,
            maximumInstanceLifetimeSeconds: BuildAgentPool.DefaultMaximumInstanceLifetimeSeconds,
            failureRetentionMinutes: BuildAgentPool.DefaultFailureRetentionMinutes,
            createdByActorId: actorId);
        var enrollment = new EdgeAgentEnrollment(
            Id: Guid.CreateVersion7(),
            PlatformId: Guid.Empty,
            ResourceType: EdgeAgentResourceType.BuildAgentPool,
            ResourceId: poolId,
            TokenHash: EdgeAgentManagementService.HashToken(token),
            ExpiresAtUtc: utcNow.AddHours(1),
            UsedAtUtc: null,
            RevokedAtUtc: null,
            CreatedByActorId: actorId,
            CreatedAtUtc: utcNow.AddMinutes(-1));

        var buildAgentPools = new Mock<IBuildAgentPoolRepository>(MockBehavior.Strict);
        buildAgentPools
            .Setup(x => x.GetAsync(poolId, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(pool);

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.GetEnrollmentByTokenHashAsync(enrollment.TokenHash, It.IsAny<CancellationToken>()))
            .ReturnsAsync(enrollment);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.BuildAgentPools).Returns(buildAgentPools.Object);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork
            .Setup(x => x.DisposeAsync())
            .Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var service = new EdgeAgentManagementService(
            provider.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Options.Create(new EdgeAgentOptions()));

        var result = await service.CompleteEnrollmentAsync(
            new EdgeAgentEnrollmentRequest(
                token,
                AgentPublicKey: Convert.ToBase64String([1, 2, 3]),
                AgentFingerprint: string.Empty,
                Hostname: "builder",
                AgentVersion: "edge-agent-test",
                CapabilitiesJson: """{"commands":["platform.checkHealth","containers.list","containers.logs"]}""",
                ProtocolVersion: 1),
            utcNow,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("images.build", error.Message);
        edgeAgents.Verify(x => x.AddBindingAsync(It.IsAny<EdgeAgentBinding>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task MarkConnectedAsync_ShouldAddPlatformConnectedActivity_WhenPlatformTransitionsOnline()
    {
        var platformId = Guid.CreateVersion7();
        var utcNow = new DateTime(2026, 7, 10, 12, 0, 0, DateTimeKind.Utc);
        var platform = Platform.FromPersistence(
            id: platformId,
            name: "edge-platform",
            address: $"edge://{platformId:D}",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 0,
            memTotal: 0,
            status: PlatformStatus.Offline,
            connectorType: PlatformConnectorType.EdgeAgent,
            platformDescriptor: new DockerPlatformDescriptor("edge", 0, 0, 0, 0));

        ActivityEvent? capturedActivity = null;

        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        platforms
            .Setup(x => x.UpdateAsync(platform, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.GetPlatformStateByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new EdgeAgentPlatformState(platform, null));
        edgeAgents
            .Setup(x => x.UpdateBindingConnectedAsync(
                EdgeAgentResourceType.Platform,
                platformId,
                utcNow,
                "edge-host",
                "edge-agent-test",
                """{"containers":true}""",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var activities = new Mock<IActivityEventRepository>(MockBehavior.Strict);
        activities
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((activity, _) => capturedActivity = activity)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>(MockBehavior.Strict);
        actors
            .Setup(x => x.GetById(Constants.SystemId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork.SetupGet(x => x.ActivityEventRepository).Returns(activities.Object);
        unitOfWork.SetupGet(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        unitOfWork
            .Setup(x => x.DisposeAsync())
            .Returns(ValueTask.CompletedTask);

        var notificationQueue = new Mock<INotificationQueue>(MockBehavior.Strict);
        notificationQueue
            .Setup(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var service = new EdgeAgentManagementService(
            provider.GetRequiredService<IServiceScopeFactory>(),
            notificationQueue.Object,
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Options.Create(new EdgeAgentOptions()));

        await service.MarkConnectedAsync(
            platformId,
            "edge-host",
            "edge-agent-test",
            """{"containers":true}""",
            utcNow,
            TestContext.Current.CancellationToken);

        Assert.Equal(PlatformStatus.Online, platform.Status);
        Assert.NotNull(capturedActivity);
        Assert.Equal(ActivityEventType.PlatformConnected, capturedActivity.EventType);
        Assert.IsType<PlatformConnected>(capturedActivity.Info);

        platforms.VerifyAll();
        edgeAgents.VerifyAll();
        activities.VerifyAll();
        actors.VerifyAll();
        notificationQueue.Verify(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()), Times.Exactly(2));
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task MarkConnectedAsync_ShouldAddPlatformConnectedActivity_WhenBindingTransitionsFromOffline()
    {
        var platformId = Guid.CreateVersion7();
        var agentId = Guid.CreateVersion7();
        var utcNow = new DateTime(2026, 7, 10, 12, 30, 0, DateTimeKind.Utc);
        var platform = Platform.FromPersistence(
            id: platformId,
            name: "edge-platform-stale",
            address: $"edge://{platformId:D}",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 0,
            memTotal: 0,
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.EdgeAgent,
            platformDescriptor: new DockerPlatformDescriptor("edge", 0, 0, 0, 0));

        var binding = new EdgeAgentBinding(
            Guid.CreateVersion7(),
            platformId,
            EdgeAgentResourceType.Platform,
            platformId,
            agentId,
            "public-key",
            "SHA256:test",
            EdgeAgentConnectionStatus.Offline,
            LastConnectedAtUtc: null,
            LastDisconnectedAtUtc: utcNow.AddMinutes(-1),
            LastHeartbeatAtUtc: null,
            LastSeenVersion: null,
            LastSeenHostname: null,
            CapabilitiesJson: null,
            ProtocolVersion: 1,
            RevokedAtUtc: null,
            CreatedAtUtc: utcNow.AddHours(-1),
            UpdatedAtUtc: utcNow.AddMinutes(-1));

        ActivityEvent? capturedActivity = null;

        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        platforms
            .Setup(x => x.UpdateAsync(platform, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var edgeAgents = new Mock<IEdgeAgentRepository>(MockBehavior.Strict);
        edgeAgents
            .Setup(x => x.GetPlatformStateByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new EdgeAgentPlatformState(platform, binding));
        edgeAgents
            .Setup(x => x.UpdateBindingConnectedAsync(
                EdgeAgentResourceType.Platform,
                platformId,
                utcNow,
                "edge-host",
                "edge-agent-test",
                """{"containers":true}""",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var activities = new Mock<IActivityEventRepository>(MockBehavior.Strict);
        activities
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((activity, _) => capturedActivity = activity)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>(MockBehavior.Strict);
        actors
            .Setup(x => x.GetById(Constants.SystemId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(x => x.EdgeAgents).Returns(edgeAgents.Object);
        unitOfWork.SetupGet(x => x.ActivityEventRepository).Returns(activities.Object);
        unitOfWork.SetupGet(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        unitOfWork
            .Setup(x => x.DisposeAsync())
            .Returns(ValueTask.CompletedTask);

        var notificationQueue = new Mock<INotificationQueue>(MockBehavior.Strict);
        notificationQueue
            .Setup(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var service = new EdgeAgentManagementService(
            provider.GetRequiredService<IServiceScopeFactory>(),
            notificationQueue.Object,
            Mock.Of<IPlatformStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Options.Create(new EdgeAgentOptions()));

        await service.MarkConnectedAsync(
            platformId,
            "edge-host",
            "edge-agent-test",
            """{"containers":true}""",
            utcNow,
            TestContext.Current.CancellationToken);

        Assert.NotNull(capturedActivity);
        Assert.Equal(ActivityEventType.PlatformConnected, capturedActivity.EventType);
        Assert.IsType<PlatformConnected>(capturedActivity.Info);

        platforms.VerifyAll();
        edgeAgents.VerifyAll();
        activities.VerifyAll();
        actors.VerifyAll();
        notificationQueue.Verify(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()), Times.Exactly(2));
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }
}
