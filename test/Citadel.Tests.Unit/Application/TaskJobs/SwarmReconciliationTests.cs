using Application.Services.Abstractions;
using Application.Services;
using Application.Services.Alerts;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class SwarmReconciliationTests
{
    [Fact]
    public async Task NodeAgentInfrastructure_ShouldRevokeBootstrap_WhenOwnedServiceIsMissing()
    {
        var platform = CreateNodeAgentPlatform();
        var now = DateTime.UtcNow;
        var installation = CreateNodeAgentInstallation(platform, now);
        var manager = CreateNode(platform.Id, "manager-node", "Manager", "manager", now);
        var worker = CreateNode(platform.Id, "worker-node", "Worker", "worker", now);
        var edgeAgents = new Mock<IEdgeAgentRepository>();
        edgeAgents.Setup(repository => repository.GetNodeAgentInstallationAsync(
                platform.Id,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(installation);
        edgeAgents.Setup(repository => repository.GetNodeBindingsAsync(
                platform.Id,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        edgeAgents.Setup(repository => repository.RevokeNodeAgentBootstrapsAsync(
                platform.Id,
                now,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = CreateNodeAgentUnitOfWork(platform, edgeAgents.Object);
        var workItem = new ReconcileSwarmNodeAgentInfrastructureWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([manager, worker], [], [], [], [], []),
            now,
            TimeSpan.FromMinutes(10),
            ["amd64"]);

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Contains("missing", workItem.ServiceDriftReason, StringComparison.OrdinalIgnoreCase);
        edgeAgents.Verify(repository => repository.RevokeNodeAgentBootstrapsAsync(
            platform.Id,
            now,
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task NodeAgentInfrastructure_ShouldRevokeOnlyAbsentBindingBeyondGrace()
    {
        var platform = CreateNodeAgentPlatform();
        var now = DateTime.UtcNow;
        var manager = CreateNode(platform.Id, "manager-node", "Manager", "manager", now);
        var oldBinding = CreateNodeBinding(platform, "removed-node", now.AddMinutes(-11));
        var recentBinding = CreateNodeBinding(platform, "recently-removed-node", now.AddMinutes(-9));
        var edgeAgents = new Mock<IEdgeAgentRepository>();
        edgeAgents.Setup(repository => repository.GetNodeAgentInstallationAsync(
                platform.Id,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(CreateNodeAgentInstallation(platform, now));
        edgeAgents.Setup(repository => repository.GetNodeBindingsAsync(
                platform.Id,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([oldBinding, recentBinding]);
        edgeAgents.Setup(repository => repository.RevokeNodeBindingAsync(
                platform.Id,
                oldBinding.DockerNodeId!,
                now,
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = CreateNodeAgentUnitOfWork(platform, edgeAgents.Object);
        var workItem = new ReconcileSwarmNodeAgentInfrastructureWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([manager], [], [], [], [], []),
            now,
            TimeSpan.FromMinutes(10),
            ["amd64"]);

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal([oldBinding.DockerNodeId], workItem.RevokedNodeIds);
        edgeAgents.Verify(repository => repository.RevokeNodeBindingAsync(
            platform.Id,
            recentBinding.DockerNodeId!,
            It.IsAny<DateTime>(),
            It.IsAny<string>(),
            It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task TimedOutStack_ShouldRecoverOnlyAfterAllOwnedServicesConverge()
    {
        var platform = CreatePlatform();
        var stack = Stack.Create(
            "demo",
            Constants.SystemId,
            StackSource.Git,
            platform.Id,
            new GitStack(
                Guid.CreateVersion7(),
                "main",
                null,
                StackUpdateBehavior.Disabled,
                ProjectName: "demo",
                ComposePaths: ["compose.yml"],
                DestroyBeforeDeploy: false),
            driftPolicy: StackDriftPolicy.Disabled,
            platform: platform);
        stack.PartialUpdate(StackReleaseStatus.TimedOut);
        var labels = new Dictionary<string, string>
        {
            ["com.docker.stack.namespace"] = "demo",
            ["com.citadel.managed"] = "true",
            ["com.citadel.stack-id"] = stack.Id.ToString("D"),
            ["com.citadel.release-id"] = stack.CurrentStackReleaseId.ToString("D"),
            ["com.citadel.stack-service-count"] = "1"
        };
        var projection = new SwarmServiceProjection(
            platform.Id, "service-id", 1, "demo_api", "Replicated", "nginx",
            1, 1, "Completed", null, [], [], [], [], labels, null, null,
            DateTimeOffset.UtcNow, false, SwarmServiceOwnership.CitadelStack, "demo");

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(repository => repository.GetInfoAsync(
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                platform.Id))
            .ReturnsAsync([stack]);
        stacks.Setup(repository => repository.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        stacks.Setup(repository => repository.GetReleasesByStackIdAsync(
                stack.Id,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack.CurrentStackRelease!]);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(repository => repository.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Stacks).Returns(stacks.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        unitOfWork.SetupGet(value => value.SwarmServices).Returns(CreateEmptyManagedServiceRepository());
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        var gitMaterializer = new Mock<IGitStackMaterializer>();
        gitMaterializer.Setup(value => value.ActivateReleaseAsync(
                stack.Id,
                stack.CurrentStackReleaseId,
                It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        gitMaterializer.Setup(value => value.PruneSnapshotsAsync(
                stack.Id,
                It.IsAny<IReadOnlyCollection<Guid>>(),
                It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [projection], [], [], [], []),
            gitStackMaterializer: gitMaterializer.Object);

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
        Assert.Same(stack, Assert.Single(workItem.RecoveredStacks));
        stacks.Verify(repository => repository.UpdateAsync(stack, It.IsAny<CancellationToken>()), Times.Once);
        gitMaterializer.Verify(value => value.ActivateReleaseAsync(
            stack.Id,
            stack.CurrentStackReleaseId,
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task HealthyStack_ShouldBecomeDegradedWhenAnOwnedServiceHasNoRunningTask()
    {
        var platform = CreatePlatform();
        var stack = Stack.Create(
            "demo",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack(
                "services:\n  api:\n    image: nginx\n",
                StackUpdateBehavior.Disabled,
                ProjectName: "demo"),
            driftPolicy: StackDriftPolicy.Disabled,
            platform: platform);
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        var labels = new Dictionary<string, string>
        {
            ["com.docker.stack.namespace"] = "demo",
            ["com.citadel.managed"] = "true",
            ["com.citadel.stack-id"] = stack.Id.ToString("D"),
            ["com.citadel.release-id"] = stack.CurrentStackReleaseId.ToString("D"),
            ["com.citadel.stack-service-count"] = "1"
        };
        var projection = new SwarmServiceProjection(
            platform.Id, "service-id", 1, "demo_api", "Replicated", "nginx",
            0, 1, "None", null, [], [], [], [], labels, null, null,
            DateTimeOffset.UtcNow, false, SwarmServiceOwnership.CitadelStack, "demo");

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(repository => repository.GetInfoAsync(
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                platform.Id))
            .ReturnsAsync([stack]);
        stacks.Setup(repository => repository.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(repository => repository.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Stacks).Returns(stacks.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        unitOfWork.SetupGet(value => value.SwarmServices).Returns(CreateEmptyManagedServiceRepository());
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [projection], [], [], [], []));

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease?.Status);
        Assert.Same(stack, Assert.Single(workItem.RecoveredStacks));
        stacks.Verify(repository => repository.UpdateAsync(stack, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task TimedOutStack_ShouldRemainRecoverableWhenRollbackResourceMetadataIsMissing()
    {
        var platform = CreatePlatform();
        var stack = Stack.Create(
            "demo",
            Constants.SystemId,
            StackSource.Git,
            platform.Id,
            new GitStack(
                Guid.CreateVersion7(),
                "main",
                null,
                StackUpdateBehavior.Disabled,
                ProjectName: "demo",
                ComposePaths: ["compose.yml"],
                DestroyBeforeDeploy: false),
            driftPolicy: StackDriftPolicy.Disabled,
            platform: platform);
        stack.PartialUpdate(StackReleaseStatus.TimedOut);
        var labels = new Dictionary<string, string>
        {
            ["com.docker.stack.namespace"] = "demo",
            ["com.citadel.managed"] = "true",
            ["com.citadel.stack-id"] = stack.Id.ToString("D"),
            ["com.citadel.release-id"] = stack.CurrentStackReleaseId.ToString("D"),
            ["com.citadel.stack-service-count"] = "1"
        };
        var projection = new SwarmServiceProjection(
            platform.Id, "service-id", 1, "demo_api", "Replicated", "nginx",
            1, 1, "Completed", null, [], [], ["secret-id"], [], labels, null, null,
            DateTimeOffset.UtcNow, false, SwarmServiceOwnership.CitadelStack, "demo");
        var secret = new SwarmSecretProjection(
            platform.Id, "secret-id", 1, "demo_secret", null, ["demo_api"],
            new Dictionary<string, string> { ["com.docker.stack.namespace"] = "demo" },
            null, null, DateTimeOffset.UtcNow, false);

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(repository => repository.GetInfoAsync(
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                platform.Id))
            .ReturnsAsync([stack]);
        stacks.Setup(repository => repository.GetReleaseSwarmResourcesAsync(
                stack.CurrentStackReleaseId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(repository => repository.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Stacks).Returns(stacks.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        unitOfWork.SetupGet(value => value.SwarmServices).Returns(CreateEmptyManagedServiceRepository());
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        var gitMaterializer = new Mock<IGitStackMaterializer>();
        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [projection], [], [], [secret], []),
            gitStackMaterializer: gitMaterializer.Object);

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(StackReleaseStatus.TimedOut, stack.CurrentStackRelease?.Status);
        Assert.Empty(workItem.RecoveredStacks);
        stacks.Verify(repository => repository.UpdateAsync(
            It.IsAny<Stack>(),
            It.IsAny<CancellationToken>()), Times.Never);
        gitMaterializer.Verify(value => value.ActivateReleaseAsync(
            It.IsAny<Guid>(),
            It.IsAny<Guid>(),
            It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task PartialConnectorFailure_ShouldKeepThePreviousSnapshotAndMarkItStale()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        connector
            .Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<IReadOnlyList<SwarmServiceResult>>(
                new InternalServerError("Services could not be read.")));
        var repository = CreateStaleRepository(platform.Id);

        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);

        var result = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
        repository.Verify(
            value => value.MarkStaleAsync(platform.Id, It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task SuccessfulRefresh_ShouldLimitThePersistedActiveTasks()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var tasks = Enumerable.Range(0, 501)
            .Select(index => new SwarmTaskResult(
                $"task-{index}", 1, $"task-{index}", string.Empty, null, string.Empty,
                "Running", "Running", null, null, "nginx:latest", [], null, null, null))
            .ToArray();
        connector
            .Setup(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>(tasks));
        SwarmProjectionSnapshot? persisted = null;
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .Callback<Guid, SwarmProjectionSnapshot, CancellationToken>((_, snapshot, _) => persisted = snapshot)
            .ReturnsAsync(501);

        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);

        var result = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.NotNull(persisted);
        Assert.Equal(500, persisted.Tasks.Count);
        connector.Verify(value => value.ListNodesAsync(
            It.Is<ListSwarmNodesCommand>(command =>
                command.Limit == SwarmInventoryLimits.AuthoritativeSnapshotItems),
            It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.ListServicesAsync(
            It.Is<ListSwarmServicesCommand>(command =>
                command.Limit == SwarmInventoryLimits.AuthoritativeSnapshotItems),
            It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.ListTasksAsync(
            It.Is<ListSwarmTasksCommand>(command => command.Limit == SwarmInventoryLimits.MaximumItems),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task NotificationFailure_ShouldNotMarkCommittedInventoryStale()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        var notifications = new Mock<INotificationQueue>();
        notifications
            .Setup(value => value.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()))
            .Returns(new ValueTask(Task.FromException(new InvalidOperationException("queue unavailable"))));

        await using var provider = CreateProvider(platform, repository.Object, notifications.Object);
        var job = CreateJob(provider, connector.Object);

        var result = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        repository.Verify(
            value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.MarkStaleAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task DifferentPlatforms_ShouldRefreshConcurrently()
    {
        var first = CreatePlatform();
        Platform second;
        do
        {
            second = CreatePlatform();
        }
        while ((uint)first.Id.GetHashCode() % 64 == (uint)second.Id.GetHashCode() % 64);

        var bothStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var starts = 0;
        var connector = CreateConnector();
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .Returns(async () =>
            {
                if (Interlocked.Increment(ref starts) == 2)
                    bothStarted.TrySetResult();
                await release.Task;
                return Result.Success<IReadOnlyList<SwarmNodeResult>>([]);
            });
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        await using var provider = CreateProvider(
            first,
            repository.Object,
            additionalPlatforms: [second]);
        var job = CreateJob(provider, connector.Object);
        var firstRefresh = job.RefreshAsync(first.Id, TestContext.Current.CancellationToken);
        var secondRefresh = job.RefreshAsync(second.Id, TestContext.Current.CancellationToken);

        try
        {
            await bothStarted.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        }
        finally
        {
            release.TrySetResult();
        }

        var results = await Task.WhenAll(firstRefresh, secondRefresh);
        Assert.All(results, result => Assert.True(result.IsSuccess()));
    }

    [Fact]
    public async Task ConcurrentInitialization_ShouldReadDockerOnlyOnce()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var initialized = false;
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => initialized ? [CreateNode(platform.Id, isStale: false)] : []);
        repository
            .Setup(value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .Callback(() => initialized = true)
            .ReturnsAsync(0);
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);

        var results = await Task.WhenAll(
            job.EnsureInitializedAsync(platform.Id, TestContext.Current.CancellationToken),
            job.EnsureInitializedAsync(platform.Id, TestContext.Current.CancellationToken));

        Assert.All(results, result => Assert.True(result.IsSuccess()));
        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task EnumerationFailure_ShouldBeContainedSoTheNextIterationCanRun()
    {
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .SetupSequence(value => value.GetPlatformsWithLatestStatAsync(It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("database temporarily unavailable"))
            .ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.Setup(value => value.DisposeAsync()).Returns(ValueTask.CompletedTask);
        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .AddSingleton(Mock.Of<IDbWorkQueue>())
            .AddSingleton(Mock.Of<INotificationQueue>())
            .BuildServiceProvider();
        var job = CreateJob(provider, CreateConnector().Object);

        await job.ReconcileOnceAsync(TestContext.Current.CancellationToken);
        await job.ReconcileOnceAsync(TestContext.Current.CancellationToken);

        platforms.Verify(
            value => value.GetPlatformsWithLatestStatAsync(It.IsAny<CancellationToken>()),
            Times.Exactly(2));
    }

    [Fact]
    public async Task DaemonEventBurst_ShouldScheduleOneAuthoritativeRefresh()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var repository = CreateWritableRepository();
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);
        var daemonEvent = new DaemonResourceEventInfo(
            "update",
            ContainerEventType.Service,
            "service-1",
            DaemonEventScope.Swarm);

        for (var index = 0; index < 20; index++)
            await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);

        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);

        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task DaemonEventDuringRefresh_ShouldScheduleOneFollowUpRefresh()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var firstRefreshStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseFirstRefresh = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var calls = 0;
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .Returns(async () =>
            {
                if (Interlocked.Increment(ref calls) == 1)
                {
                    firstRefreshStarted.TrySetResult();
                    await releaseFirstRefresh.Task;
                }
                return Result.Success<IReadOnlyList<SwarmNodeResult>>([]);
            });
        var repository = CreateWritableRepository();
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);
        var daemonEvent = new DaemonResourceEventInfo(
            "update",
            ContainerEventType.Node,
            "node-1",
            DaemonEventScope.Swarm);

        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        var firstBatch = job.ReconcilePendingAsync(TestContext.Current.CancellationToken);
        await firstRefreshStarted.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        releaseFirstRefresh.TrySetResult();
        await firstBatch;
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);

        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Exactly(2));
    }

    [Fact]
    public async Task ContainerEvent_ShouldRefreshOnlyAfterThePlatformIsKnownToBeSwarm()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var repository = CreateWritableRepository();
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);
        var daemonEvent = new DaemonContainerEventInfo(
            "die",
            "task-container-1",
            null,
            DaemonEventScope.Local);

        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);
        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        var initialization = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);
        Assert.True(initialization.IsSuccess());
        connector.Invocations.Clear();

        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);
        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task BuilderPruneEvent_ShouldNotRefreshSwarmInventory()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        await using var provider = CreateProvider(platform, CreateWritableRepository().Object);
        var job = CreateJob(provider, connector.Object);

        await job.NotifyDaemonEventAsync(
            platform.Id,
            new DaemonResourceEventInfo(
                "prune",
                ContainerEventType.Builder,
                string.Empty,
                DaemonEventScope.Local),
            TestContext.Current.CancellationToken);
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);

        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task CompleteSnapshot_ShouldReplaceAndCommit()
    {
        var platformId = Guid.CreateVersion7();
        var nodes = new[] { CreateNode(platformId, isStale: false) };
        var snapshot = CreateSnapshot(nodes);
        SwarmProjectionSnapshot? persisted = null;
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                platformId,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .Callback<Guid, SwarmProjectionSnapshot, CancellationToken>((_, value, _) => persisted = value)
            .ReturnsAsync(nodes.Length);
        var committed = false;
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Swarm).Returns(repository.Object);
        unitOfWork.SetupGet(value => value.SwarmServices).Returns(CreateEmptyManagedServiceRepository());
        unitOfWork
            .Setup(value => value.CommitAsync(It.IsAny<CancellationToken>()))
            .Callback(() => committed = true)
            .Returns(Task.CompletedTask);
        await new PersistSwarmSnapshotWorkItem(
                platformId,
                snapshot)
            .ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        repository.Verify(
            value => value.ReplaceAsync(
                platformId,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.MarkStaleAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
        Assert.True(committed);
        Assert.Equal(nodes, persisted?.Nodes);
        unitOfWork.Verify(value => value.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task FailedSnapshot_ShouldKeepRowsAndMarkThemStale()
    {
        var platformId = Guid.CreateVersion7();
        var staleNodes = new[] { CreateNode(platformId, isStale: true) };
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.MarkStaleAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        repository
            .Setup(value => value.GetNodesAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(staleNodes);
        repository.Setup(value => value.GetServicesAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetTasksAsync(platformId, 500, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetNetworksAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetSecretsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetConfigsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Swarm).Returns(repository.Object);
        unitOfWork
            .Setup(value => value.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        await new PersistSwarmSnapshotWorkItem(
                platformId,
                snapshot: null)
            .ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        repository.Verify(
            value => value.MarkStaleAsync(platformId, It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        unitOfWork.Verify(value => value.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public void CompletePostGraceSnapshot_ShouldProveOnlyUnchangedAmbiguousOperationsNotAccepted()
    {
        var now = DateTimeOffset.UtcNow;
        var operation = new SwarmServiceOperation(
            Guid.CreateVersion7(),
            SwarmServiceOperationKind.Apply,
            SwarmServiceOperationState.OutcomeUnknown,
            BaseDockerVersion: 4,
            TargetDesiredSpecHash: "desired",
            TargetRuntimeHash: "runtime",
            TargetRowVersion: 1,
            ExpectedForceUpdate: null,
            PreparedAt: now.AddMinutes(-3).UtcDateTime,
            AttemptedAt: now.AddMinutes(-2).UtcDateTime,
            CompletedAt: now.AddSeconds(-31).UtcDateTime,
            ClusterId: "cluster-a",
            ActorId: Constants.SystemId);

        Assert.True(PersistSwarmSnapshotWorkItem.CanProveNotAccepted(
            operation,
            CreateServiceProjection(version: 4),
            "cluster-a",
            now));
        Assert.False(PersistSwarmSnapshotWorkItem.CanProveNotAccepted(
            operation,
            CreateServiceProjection(version: 5),
            "cluster-a",
            now));
        Assert.False(PersistSwarmSnapshotWorkItem.CanProveNotAccepted(
            operation,
            CreateServiceProjection(version: 4),
            "another-cluster",
            now));
        Assert.False(PersistSwarmSnapshotWorkItem.CanProveNotAccepted(
            operation,
            CreateServiceProjection(version: 4),
            "cluster-a",
            now.AddSeconds(-2)));
    }

    [Fact]
    public void AcceptedOperationAfterGrace_ShouldFailWhenObservedRuntimeDoesNotMatch()
    {
        var now = DateTimeOffset.UtcNow;
        var operationId = Guid.CreateVersion7();
        var operation = new SwarmServiceOperation(
            operationId,
            SwarmServiceOperationKind.Apply,
            SwarmServiceOperationState.Accepted,
            BaseDockerVersion: 4,
            TargetDesiredSpecHash: "desired",
            TargetRuntimeHash: "expected-runtime",
            TargetRowVersion: 1,
            ExpectedForceUpdate: null,
            PreparedAt: now.AddMinutes(-4).UtcDateTime,
            AttemptedAt: now.AddMinutes(-3).UtcDateTime,
            ClusterId: "cluster-a",
            ActorId: Constants.SystemId);
        var live = CreateServiceProjection(version: 5) with
        {
            Labels = new Dictionary<string, string>
            {
                ["com.citadel.operation-id"] = operationId.ToString()
            },
            LiveRuntimeHash = "different-runtime"
        };

        Assert.True(PersistSwarmSnapshotWorkItem.TryGetUnresolvedOperationFailure(
            operation,
            live,
            "cluster-a",
            now,
            out var resultCode,
            out _));
        Assert.Equal("AcceptedRuntimeMismatch", resultCode);
        Assert.False(PersistSwarmSnapshotWorkItem.TryGetUnresolvedOperationFailure(
            operation,
            live,
            "cluster-a",
            now.AddMinutes(-1),
            out _,
            out _));
    }

    [Fact]
    public void AcceptedOperationAfterGrace_ShouldFailWhenRuntimeIsMissing()
    {
        var now = DateTimeOffset.UtcNow;
        var operation = new SwarmServiceOperation(
            Guid.CreateVersion7(),
            SwarmServiceOperationKind.Apply,
            SwarmServiceOperationState.Accepted,
            BaseDockerVersion: null,
            TargetDesiredSpecHash: "desired",
            TargetRuntimeHash: "expected-runtime",
            TargetRowVersion: 1,
            ExpectedForceUpdate: null,
            PreparedAt: now.AddMinutes(-4).UtcDateTime,
            AttemptedAt: now.AddMinutes(-3).UtcDateTime,
            ClusterId: "cluster-a",
            ActorId: Constants.SystemId);

        Assert.True(PersistSwarmSnapshotWorkItem.TryGetUnresolvedOperationFailure(
            operation,
            live: null,
            "cluster-a",
            now,
            out var resultCode,
            out _));
        Assert.Equal("RuntimeMissingAfterAcceptance", resultCode);
    }

    [Fact]
    public async Task PausedManagedRollout_ShouldPersistTaskErrorAndFailureActivity()
    {
        var platform = CreatePlatform();
        var service = CreateAcceptedManagedService(platform.Id);
        var live = CreateManagedServiceProjection(service, "Paused", running: 1, desired: 2);
        const string taskError = "starting container failed: strconv.Atoi: parsing \"\": invalid syntax";
        var task = new SwarmTaskProjection(
            platform.Id,
            "task-failed",
            1,
            "managed-service.2",
            live.DockerServiceId,
            service.Name,
            2,
            "node-1",
            "worker-1",
            "Running",
            "Failed",
            "starting container failed",
            taskError,
            live.Image,
            [],
            DateTimeOffset.UtcNow,
            DateTimeOffset.UtcNow,
            DateTimeOffset.UtcNow,
            DateTimeOffset.UtcNow,
            false);
        var activities = new List<ActivityEvent>();
        var unitOfWork = CreateManagedUnitOfWork(platform, service, activities);

        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [live], [task], [], [], []));
        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(SwarmServiceHealth.Failed, service.Health);
        Assert.Equal(ResourceControlState.Idle, service.ControlState);
        Assert.Equal(SwarmServiceOperationState.Rejected, service.CurrentOperation?.State);
        Assert.Equal("RolloutPaused", service.CurrentOperation?.ResultCode);
        Assert.Equal(taskError, service.CurrentOperation?.ResultMessage);
        var activity = Assert.Single(activities);
        Assert.Equal(ActivityEventType.SwarmServiceOperationFailed, activity.EventType);
        Assert.Equal(ActivityStatus.Failure, activity.Status);
        Assert.Equal(taskError, Assert.IsType<SwarmServiceOperationFailed>(activity.Info).Reason);
        var alert = Assert.Single(workItem.OperationFailures);
        Assert.Equal(service.Id, alert.Id);
        Assert.Equal(service.CurrentOperation?.Id, alert.OperationId);
        Assert.Equal(taskError, alert.Reason);
    }

    [Fact]
    public async Task UpdatingManagedRollout_ShouldRemainAcceptedWithoutSuccessActivity()
    {
        var platform = CreatePlatform();
        var service = CreateAcceptedManagedService(platform.Id);
        var live = CreateManagedServiceProjection(service, "Updating", running: 2, desired: 2);
        var activities = new List<ActivityEvent>();
        var unitOfWork = CreateManagedUnitOfWork(platform, service, activities);

        await new PersistSwarmSnapshotWorkItem(
                platform.Id,
                new SwarmProjectionSnapshot([], [live], [], [], [], []))
            .ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(ResourceControlState.Processing, service.ControlState);
        Assert.Equal(SwarmServiceOperationState.Accepted, service.CurrentOperation?.State);
        Assert.Empty(activities);
    }

    [Fact]
    public async Task CompletedHealthyRollout_ShouldCompleteOperationAndRecordSuccessActivity()
    {
        var platform = CreatePlatform();
        var service = CreateAcceptedManagedService(platform.Id);
        var live = CreateManagedServiceProjection(service, "Completed", running: 2, desired: 2);
        var activities = new List<ActivityEvent>();
        var unitOfWork = CreateManagedUnitOfWork(platform, service, activities);

        await new PersistSwarmSnapshotWorkItem(
                platform.Id,
                new SwarmProjectionSnapshot([], [live], [], [], [], []))
            .ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(ResourceControlState.Idle, service.ControlState);
        Assert.Equal(SwarmServiceOperationState.Completed, service.CurrentOperation?.State);
        Assert.Equal(live.LiveRuntimeHash, service.LastAppliedRuntimeHash);
        var activity = Assert.Single(activities);
        Assert.Equal(ActivityEventType.SwarmServiceApplied, activity.EventType);
        Assert.Equal(ActivityStatus.Success, activity.Status);
    }

    [Fact]
    public async Task AdoptedServiceWithoutOwnershipLabels_ShouldRemainLinkedByDockerIdentity()
    {
        var platform = CreatePlatform();
        var spec = new SwarmServiceSpec
        {
            Image = new SwarmExternalImage(Guid.CreateVersion7(), "nginx:latest"),
            SchedulingMode = SwarmServiceSchedulingMode.Replicated,
            Replicas = 1
        };
        var service = SwarmService.AdoptExisting(
            "adopted-service",
            "Adopted from Docker Swarm Service service.",
            platform.Id,
            Constants.SystemId,
            "service",
            "docker-service",
            4,
            "old-runtime",
            spec);
        var live = CreateServiceProjection(5) with
        {
            PlatformId = platform.Id,
            Ownership = SwarmServiceOwnership.Unmanaged,
            SwarmServiceId = null,
            Labels = new Dictionary<string, string>()
        };
        var activities = new List<ActivityEvent>();
        var unitOfWork = CreateManagedUnitOfWork(platform, service, activities);
        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [live], [], [], [], []));

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        var associated = Assert.Single(workItem.Current.Services);
        Assert.Equal(service.Id, associated.SwarmServiceId);
        Assert.Equal(SwarmServiceOwnership.CitadelService, associated.Ownership);
        Assert.Same(associated, service.Projection);
        Assert.Equal(SwarmServiceHealth.Healthy, service.Health);
        Assert.Equal(SwarmServiceSynchronizationState.DesiredChangesPending, service.SynchronizationState);
        Assert.Empty(activities);
    }

    [Fact]
    public async Task MissingManagedServiceOwner_ShouldBecomeAdoptable()
    {
        var platform = CreatePlatform();
        var orphanedOwnerId = Guid.CreateVersion7();
        var projection = CreateServiceProjection(1) with
        {
            PlatformId = platform.Id,
            Ownership = SwarmServiceOwnership.CitadelService,
            SwarmServiceId = orphanedOwnerId,
            Labels = new Dictionary<string, string>
            {
                ["com.citadel.managed"] = "true",
                ["com.citadel.service-id"] = orphanedOwnerId.ToString("D")
            }
        };
        var unitOfWork = CreateOwnershipUnitOfWork(platform.Id, [], []);
        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [projection], [], [], [], []));

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        var normalized = Assert.Single(workItem.Current.Services);
        Assert.Equal(SwarmServiceOwnership.Unmanaged, normalized.Ownership);
        Assert.Null(normalized.SwarmServiceId);
        Assert.Contains("can be adopted", normalized.OwnershipDiagnostic);
    }

    [Fact]
    public async Task MissingStackOwner_ShouldBecomeImportable()
    {
        var platform = CreatePlatform();
        var orphanedOwnerId = Guid.CreateVersion7();
        var projection = CreateServiceProjection(1) with
        {
            PlatformId = platform.Id,
            Name = "demo_web",
            Ownership = SwarmServiceOwnership.CitadelStack,
            DockerStackNamespace = "demo",
            SwarmServiceId = null,
            Labels = new Dictionary<string, string>
            {
                ["com.docker.stack.namespace"] = "demo",
                ["com.citadel.managed"] = "true",
                ["com.citadel.stack-id"] = orphanedOwnerId.ToString("D")
            }
        };
        var unitOfWork = CreateOwnershipUnitOfWork(platform.Id, [], []);
        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [projection], [], [], [], []));

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        var normalized = Assert.Single(workItem.Current.Services);
        Assert.Equal(SwarmServiceOwnership.DockerStackExternal, normalized.Ownership);
        Assert.Null(normalized.StackId);
        Assert.Contains("can be imported", normalized.OwnershipDiagnostic);
    }

    [Fact]
    public async Task ImportedStackAssociation_ShouldOverrideStaleDockerOwnerLabel()
    {
        var platform = CreatePlatform();
        var stack = Stack.Create(
            "demo",
            Constants.SystemId,
            StackSource.Git,
            platform.Id,
            new GitStack(
                Guid.CreateVersion7(),
                "main",
                null,
                StackUpdateBehavior.Disabled,
                ProjectName: "demo",
                ComposePaths: ["compose.yml"],
                DestroyBeforeDeploy: false),
            driftPolicy: StackDriftPolicy.Disabled,
            platform: platform);
        var oldOwnerId = Guid.CreateVersion7();
        var live = CreateServiceProjection(1) with
        {
            PlatformId = platform.Id,
            Name = "demo_web",
            Ownership = SwarmServiceOwnership.CitadelStack,
            DockerStackNamespace = "demo",
            SwarmServiceId = null,
            Labels = new Dictionary<string, string>
            {
                ["com.docker.stack.namespace"] = "demo",
                ["com.citadel.managed"] = "true",
                ["com.citadel.stack-id"] = oldOwnerId.ToString("D")
            }
        };
        var persisted = live with { StackId = stack.Id };
        var unitOfWork = CreateOwnershipUnitOfWork(platform.Id, [persisted], [stack]);
        var workItem = new PersistSwarmSnapshotWorkItem(
            platform.Id,
            new SwarmProjectionSnapshot([], [live], [], [], [], []));

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        var normalized = Assert.Single(workItem.Current.Services);
        Assert.Equal(SwarmServiceOwnership.CitadelStack, normalized.Ownership);
        Assert.Equal(stack.Id, normalized.StackId);
        Assert.Null(normalized.OwnershipDiagnostic);
    }

    [Theory]
    [InlineData(SwarmServiceOperationState.PendingAcceptance)]
    [InlineData(SwarmServiceOperationState.OutcomeUnknown)]
    public void AmbiguousDispatchedOperationAfterGrace_ShouldNotRemainPendingForever(
        SwarmServiceOperationState state)
    {
        var now = DateTimeOffset.UtcNow;
        var operationId = Guid.CreateVersion7();
        var operation = new SwarmServiceOperation(
            operationId,
            SwarmServiceOperationKind.Apply,
            state,
            BaseDockerVersion: 4,
            TargetDesiredSpecHash: "desired",
            TargetRuntimeHash: "expected-runtime",
            TargetRowVersion: 1,
            ExpectedForceUpdate: null,
            PreparedAt: now.AddMinutes(-5).UtcDateTime,
            AttemptedAt: now.AddMinutes(-4).UtcDateTime,
            CompletedAt: state == SwarmServiceOperationState.OutcomeUnknown
                ? now.AddMinutes(-3).UtcDateTime
                : null,
            ClusterId: "cluster-a",
            ActorId: Constants.SystemId);
        var live = CreateServiceProjection(version: 5) with
        {
            Labels = new Dictionary<string, string>
            {
                ["com.citadel.operation-id"] = operationId.ToString()
            },
            LiveRuntimeHash = "different-runtime"
        };

        Assert.True(PersistSwarmSnapshotWorkItem.TryGetUnresolvedOperationFailure(
            operation,
            live,
            "cluster-a",
            now,
            out var resultCode,
            out _));
        Assert.Equal("AcceptedRuntimeMismatch", resultCode);
    }

    [Theory]
    [InlineData(SwarmServiceOperationState.PendingAcceptance, true)]
    [InlineData(SwarmServiceOperationState.Accepted, true)]
    [InlineData(SwarmServiceOperationState.OutcomeUnknown, true)]
    [InlineData(SwarmServiceOperationState.Prepared, false)]
    [InlineData(SwarmServiceOperationState.Rejected, false)]
    public void AbsentRuntime_ShouldSatisfyDispatchedDelete(
        SwarmServiceOperationState state,
        bool expected)
    {
        var operation = new SwarmServiceOperation(
            Guid.CreateVersion7(),
            SwarmServiceOperationKind.Delete,
            state,
            BaseDockerVersion: 4,
            TargetDesiredSpecHash: "desired",
            TargetRuntimeHash: null,
            TargetRowVersion: 1,
            ExpectedForceUpdate: null,
            PreparedAt: DateTime.UtcNow,
            AttemptedAt: DateTime.UtcNow,
            ClusterId: "cluster-a",
            ActorId: Constants.SystemId);

        Assert.Equal(expected, PersistSwarmSnapshotWorkItem.IsDeleteSatisfied(operation, live: null));
        Assert.False(PersistSwarmSnapshotWorkItem.IsDeleteSatisfied(
            operation,
            CreateServiceProjection(version: 5)));
    }

    private static SwarmProjectionSnapshot CreateSnapshot(IReadOnlyList<SwarmNodeProjection> nodes) =>
        new(nodes, [], [], [], [], []);

    private static Mock<ISwarmConnector> CreateConnector()
    {
        var connector = new Mock<ISwarmConnector>();
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNodeResult>>([]));
        connector
            .Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmServiceResult>>([]));
        connector
            .Setup(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>([]));
        connector
            .Setup(value => value.ListNetworksAsync(It.IsAny<ListSwarmNetworksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNetworkResult>>([]));
        connector
            .Setup(value => value.ListSecretsAsync(It.IsAny<ListSwarmSecretsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmSecretResult>>([]));
        connector
            .Setup(value => value.ListConfigsAsync(It.IsAny<ListSwarmConfigsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmConfigResult>>([]));
        return connector;
    }

    private static Mock<ISwarmProjectionRepository> CreateStaleRepository(Guid platformId)
    {
        var repository = new Mock<ISwarmProjectionRepository>();
        repository.Setup(value => value.MarkStaleAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync(0);
        repository.Setup(value => value.GetNodesAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetServicesAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetTasksAsync(platformId, 500, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetNetworksAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetSecretsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetConfigsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        return repository;
    }

    private static Mock<ISwarmProjectionRepository> CreateWritableRepository()
    {
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        return repository;
    }

    private static ServiceProvider CreateProvider(
        Platform platform,
        ISwarmProjectionRepository repository,
        INotificationQueue? notificationQueue = null,
        IReadOnlyList<Platform>? additionalPlatforms = null)
    {
        var allPlatforms = additionalPlatforms is null
            ? [platform]
            : additionalPlatforms.Prepend(platform).ToArray();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(value => value.GetByIdAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((Guid id, CancellationToken _) => allPlatforms.SingleOrDefault(value => value.Id == id));
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(repository);
        unitOfWork.SetupGet(value => value.SwarmServices).Returns(CreateEmptyManagedServiceRepository());
        unitOfWork.SetupGet(value => value.EdgeAgents).Returns(Mock.Of<IEdgeAgentRepository>());
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        unitOfWork.Setup(value => value.DisposeAsync()).Returns(ValueTask.CompletedTask);

        return new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .AddSingleton<IDbWorkQueue>(new InlineDbWorkQueue(unitOfWork.Object))
            .AddSingleton(notificationQueue ?? Mock.Of<INotificationQueue>(queue =>
                queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()) == ValueTask.CompletedTask))
            .BuildServiceProvider();
    }

    private static SwarmReconciliationJob CreateJob(ServiceProvider provider, ISwarmConnector connector)
    {
        var connectorFactory = new Mock<IConnectorFactory<ISwarmConnector>>();
        connectorFactory.Setup(value => value.GetConnector(PlatformConnectorType.Local)).Returns(connector);
        return new SwarmReconciliationJob(
            provider.GetRequiredService<IServiceScopeFactory>(),
            connectorFactory.Object,
            provider.GetRequiredService<IDbWorkQueue>(),
            provider.GetRequiredService<INotificationQueue>(),
            Mock.Of<IApplicationHubDispatcher>(),
            Mock.Of<IEdgeAgentSessionTerminator>(),
            Mock.Of<IGitStackMaterializer>(),
            Mock.Of<IAlertService>(),
            Microsoft.Extensions.Options.Options.Create(new global::Application.Configs.EdgeAgentOptions()),
            TimeProvider.System,
            Mock.Of<ILogger<SwarmReconciliationJob>>());
    }

    private static Platform CreatePlatform() => new(
        "swarm",
        "unix:///var/run/docker.sock",
        0,
        0,
        0,
        1,
        1024,
        "28.0",
        null,
        PlatformStatus.Online,
        PlatformConnectorType.Local,
        new DockerSwarmPlatformDescriptor(
            "node-1", "10.0.0.1", "Active", true, 1, 1, "daemon-1", 0, 0, 0, 0));

    private static Platform CreateNodeAgentPlatform() => new(
        "swarm-node-agent",
        "unix:///var/run/docker.sock",
        0,
        0,
        0,
        1,
        1024,
        "29.0",
        null,
        PlatformStatus.Online,
        PlatformConnectorType.Local,
        new DockerSwarmPlatformDescriptor(
            NodeID: "manager-node",
            NodeAddr: "10.0.0.1",
            LocalNodeState: "Active",
            ControlAvailable: true,
            Nodes: 2,
            Managers: 1,
            DaemonId: "manager-daemon",
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0,
            ClusterId: "cluster-test"),
        clusterId: "cluster-test");

    private static SwarmNodeProjection CreateNode(
        Guid platformId,
        string dockerNodeId,
        string role,
        string hostname,
        DateTime observedAt) => new(
        platformId,
        dockerNodeId,
        1,
        hostname,
        role,
        role.Equals("Manager", StringComparison.OrdinalIgnoreCase),
        role.Equals("Manager", StringComparison.OrdinalIgnoreCase) ? "Reachable" : string.Empty,
        "Ready",
        null,
        "Active",
        "29.0",
        "linux",
        "amd64",
        "10.0.0.2",
        new Dictionary<string, string>(),
        0,
        0,
        observedAt,
        observedAt,
        observedAt,
        false);

    private static SwarmNodeAgentInstallation CreateNodeAgentInstallation(Platform platform, DateTime now) => new(
        platform.Id,
        platform.ClusterId!,
        "manager-node",
        "manager-daemon",
        "node-agent-service",
        $"citadel-node-agent-{platform.Id:N}",
        "ghcr.io/citadel-p/citadel.agent:latest",
        "sha256:test",
        null,
        null,
        SwarmNodeAgentDesiredState.Installed,
        Guid.CreateVersion7(),
        SwarmNodeAgentOperationKind.Install,
        SwarmNodeAgentOperationState.Completed,
        now,
        Constants.SystemId,
        null,
        now,
        now);

    private static EdgeAgentBinding CreateNodeBinding(Platform platform, string dockerNodeId, DateTime observedAt) => new(
        Guid.CreateVersion7(),
        platform.Id,
        EdgeAgentResourceType.Platform,
        platform.Id,
        Guid.CreateVersion7(),
        "public-key",
        $"SHA256:{Guid.NewGuid():N}",
        EdgeAgentConnectionStatus.Offline,
        observedAt,
        observedAt,
        observedAt,
        "test",
        dockerNodeId,
        "{}",
        Constants.EdgeAgentProtocolVersion,
        null,
        observedAt,
        observedAt,
        EdgeAgentProfile.SwarmNode,
        platform.ClusterId,
        dockerNodeId,
        $"daemon-{dockerNodeId}");

    private static Mock<IUnitOfWork> CreateNodeAgentUnitOfWork(
        Platform platform,
        IEdgeAgentRepository edgeAgents)
    {
        var platforms = new Mock<IPlatformRepository>();
        platforms.Setup(repository => repository.GetByIdAsync(
                platform.Id,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.EdgeAgents).Returns(edgeAgents);
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static SwarmNodeProjection CreateNode(Guid platformId, bool isStale) =>
        new(
            platformId,
            "node-1",
            1,
            "manager-1",
            "Manager",
            true,
            "Reachable",
            "Ready",
            null,
            "Active",
            "28.0",
            "linux",
            "x86_64",
            "10.0.0.1",
            new Dictionary<string, string>(),
            1,
            1,
            null,
            null,
            DateTimeOffset.UtcNow,
            isStale);

    private static SwarmServiceProjection CreateServiceProjection(long version) => new(
        Guid.CreateVersion7(),
        "docker-service",
        version,
        "service",
        "Replicated",
        "nginx@sha256:applied",
        1,
        1,
        "None",
        null,
        [],
        [],
        [],
        [],
        new Dictionary<string, string>(),
        null,
        null,
        DateTimeOffset.UtcNow,
        false,
        SwarmServiceOwnership.CitadelService,
        LiveRuntimeHash: "old-runtime");

    private static SwarmService CreateAcceptedManagedService(Guid platformId)
    {
        var service = new SwarmService(
            "managed-service",
            platformId,
            Constants.SystemId,
            new SwarmServiceSpec
            {
                Image = new SwarmExternalImage(Guid.CreateVersion7(), "nginx:latest"),
                SchedulingMode = SwarmServiceSchedulingMode.Replicated,
                Replicas = 2
            });
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId,
            targetRuntimeHash: "target-runtime",
            baseDockerVersion: 4,
            clusterId: "cluster-a"));
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service", 4);
        return service;
    }

    private static SwarmServiceProjection CreateManagedServiceProjection(
        SwarmService service,
        string updateState,
        int running,
        int desired) =>
        new(
            service.PlatformId,
            "docker-service",
            5,
            service.DockerName,
            "Replicated",
            "nginx@sha256:applied",
            running,
            desired,
            updateState,
            updateState == "Paused" ? "update paused after a Task failed" : null,
            [],
            [],
            [],
            [],
            new Dictionary<string, string>
            {
                ["com.citadel.managed"] = "true",
                ["com.citadel.service-id"] = service.Id.ToString(),
                ["com.citadel.operation-id"] = service.CurrentOperation!.Id.ToString()
            },
            null,
            null,
            DateTimeOffset.UtcNow,
            false,
            SwarmServiceOwnership.CitadelService,
            SwarmServiceId: service.Id,
            LiveRuntimeHash: "target-runtime");

    private static Mock<IUnitOfWork> CreateManagedUnitOfWork(
        Platform platform,
        SwarmService service,
        List<ActivityEvent> activities)
    {
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(value => value.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var managedServices = new Mock<ISwarmServiceRepository>();
        managedServices
            .Setup(value => value.GetByPlatformAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([service]);
        managedServices
            .Setup(value => value.UpdateAsync(service, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm
            .Setup(value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(value => value.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((activity, _) => activities.Add(activity))
            .ReturnsAsync(1);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.SwarmServices).Returns(managedServices.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        unitOfWork.SetupGet(value => value.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static Mock<IUnitOfWork> CreateOwnershipUnitOfWork(
        Guid platformId,
        IReadOnlyList<SwarmServiceProjection> persistedServices,
        IReadOnlyList<Stack> stacks)
    {
        var stackRepository = new Mock<IStackRepository>();
        stackRepository.Setup(repository => repository.GetInfoAsync(
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                platformId))
            .ReturnsAsync(stacks);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(repository => repository.GetServicesAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(persistedServices);
        swarm.Setup(repository => repository.ReplaceAsync(
                platformId,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Stacks).Returns(stackRepository.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        unitOfWork.SetupGet(value => value.SwarmServices).Returns(CreateEmptyManagedServiceRepository());
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static ISwarmServiceRepository CreateEmptyManagedServiceRepository()
    {
        var repository = new Mock<ISwarmServiceRepository>();
        repository
            .Setup(value => value.GetByPlatformAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        return repository.Object;
    }

    private sealed class InlineDbWorkQueue(IUnitOfWork unitOfWork) : IDbWorkQueue
    {
        public System.Threading.Channels.ChannelReader<IDbWorkItem> Reader =>
            throw new NotSupportedException();

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken) =>
            EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken) =>
            await item.ExecuteAsync(unitOfWork, cancellationToken);
    }
}
