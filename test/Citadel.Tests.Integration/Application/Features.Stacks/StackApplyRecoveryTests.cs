using System.Collections.Concurrent;
using System.Collections.Immutable;
using System.Runtime.CompilerServices;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public class StackApplyRecoveryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IStackConnector> stackConnector = new();
    private readonly Mock<IContainerConnector> containerConnector = new();
    private readonly Mock<IConnectorFactory<IStackConnector>> stackConnectorFactory = new();
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerConnectorFactory = new();
    private readonly ConcurrentQueue<IReadOnlyDictionary<string, DockerContainer>> containerResults = new();
    private readonly TestStackOperationBarrier operationBarrier = new();
    private Guid platformId;
    private Guid stackId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IDbWorkQueue>();
        services.RemoveAll<IStackOperationBarrier>();
        services.RemoveAll<IConnectorFactory<IStackConnector>>();
        services.RemoveAll<IConnectorFactory<IContainerConnector>>();
        services.RemoveAll<IResourceBindingResolver>();

        services
            .AddHostedService<DbWriteWorker>()
            .AddSingleton<IDbWorkQueue, DbWorkQueue>()
            .AddSingleton<IStackOperationBarrier>(operationBarrier)
            .AddSingleton(stackConnectorFactory.Object)
            .AddSingleton(containerConnectorFactory.Object)
            .AddSingleton<IResourceBindingResolver>(
                new EmptyResourceBindingResolver());

        stackConnectorFactory
            .Setup(factory => factory.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(stackConnector.Object);
        containerConnectorFactory
            .Setup(factory => factory.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(containerConnector.Object);
        stackConnector
            .Setup(connector => connector.StackApplyAsync(
                It.IsAny<StackApplyCommand>(),
                It.IsAny<CancellationToken>()))
            .Returns(SuccessfulApplyStream());
        containerConnector
            .Setup(connector => connector.ListContainersAsync(
                It.IsAny<ContainerFilterCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(() =>
            {
                containerResults.TryDequeue(out var containers);
                return Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                    containers ?? new Dictionary<string, DockerContainer>());
            });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var stack = Stack.Create(
            name: "recovery-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            platformId: platform.Id,
            spec: new ManualStack(
                "services:\n  app:\n    image: nginx:latest\n",
                StackUpdateBehavior.Disabled,
                ProjectName: "recovery-stack"));

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
        stackId = stack.Id;
        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            platform.Id,
            new PlatformCacheEntry(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                ImmutableDictionary<string, Guid>.Empty));
    }

    [Fact]
    public async Task ConcurrentApply_AllowsOnlyOneExternalExecution()
    {
        await SetCurrentReleaseStatusAsync(StackReleaseStatus.Healthy);
        operationBarrier.BlockAt(StackOperationCheckpoint.ProcessingClaimed);
        containerResults.Enqueue(EmptyContainers());
        containerResults.Enqueue(EmptyContainers());
        containerResults.Enqueue(Containers(Container("container-first")));

        var firstApply = ApplyAsync();
        await operationBarrier.WaitUntilReachedAsync(TestContext.Current.CancellationToken);

        var secondResult = await ApplyAsync();
        Assert.Contains(
            secondResult,
            item => item.Message?.Contains("already being processed", StringComparison.OrdinalIgnoreCase) == true);
        stackConnector.Verify(
            connector => connector.StackApplyAsync(
                It.IsAny<StackApplyCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Never);

        operationBarrier.Release();
        var firstResult = await firstApply;

        Assert.Contains(firstResult, item => item.ProgressMessage == "Stack applied successfully.");
        stackConnector.Verify(
            connector => connector.StackApplyAsync(
                It.IsAny<StackApplyCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Once);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        var releases = await uow.Stacks.GetReleasesByStackIdAsync(
            stackId,
            TestContext.Current.CancellationToken);

        Assert.Equal(ResourceControlState.Idle, stack?.ControlState);
        Assert.Equal(StackReleaseStatus.Healthy, stack?.CurrentStackRelease?.Status);
        Assert.Single(releases);
    }

    [Fact]
    public async Task ReconciledOperation_CannotOverwriteNewerApply()
    {
        operationBarrier.BlockAt(StackOperationCheckpoint.ExternalStateCaptured);
        containerResults.Enqueue(EmptyContainers());
        containerResults.Enqueue(Containers(Container("container-stale")));
        containerResults.Enqueue(EmptyContainers());
        containerResults.Enqueue(Containers(Container("container-current")));

        var staleApply = ApplyAsync();
        await operationBarrier.WaitUntilReachedAsync(TestContext.Current.CancellationToken);

        try
        {
            await ReconcileCurrentOperationAsync();
            var currentResult = await ApplyAsync();
            Assert.Contains(currentResult, item => item.ProgressMessage == "Stack applied successfully.");
        }
        finally
        {
            operationBarrier.Release();
        }

        var staleResult = await staleApply;
        Assert.Contains(
            staleResult,
            item => item.Message?.Contains("superseded", StringComparison.OrdinalIgnoreCase) == true);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        var containers = await uow.Containers.GetByPlatformIdAsync(
            platformId,
            TestContext.Current.CancellationToken);
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            stackId,
            ActivityResourceType.Stack,
            ActivityEventType.StackApplied,
            page: 1,
            pageSize: 10,
            TestContext.Current.CancellationToken);

        Assert.Equal(ResourceControlState.Idle, stack?.ControlState);
        Assert.Equal(StackReleaseStatus.Healthy, stack?.CurrentStackRelease?.Status);
        Assert.Equal("container-current", Assert.Single(containers).DockerContainerId);
        Assert.Single(activities.Items);
        stackConnector.Verify(
            connector => connector.StackApplyAsync(
                It.IsAny<StackApplyCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Exactly(2));
    }

    [Fact]
    public async Task ActiveOperation_WithDisconnectedPlatform_IsNotMarkedFailed()
    {
        await MarkCurrentOperationProcessingAsync();
        Assert.True(Services.GetRequiredService<IPlatformContainerCache>().EvictPlatform(platformId));

        var result = await ApplyAsync();

        Assert.Contains(
            result,
            item => item.Message?.Contains("already being processed", StringComparison.OrdinalIgnoreCase) == true);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);

        Assert.Equal(ResourceControlState.Processing, stack?.ControlState);
        Assert.Equal(StackReleaseStatus.Applying, stack?.CurrentStackRelease?.Status);
        stackConnector.Verify(
            connector => connector.StackApplyAsync(
                It.IsAny<StackApplyCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private async Task<List<StackStreamItem>> ApplyAsync()
    {
        var result = new List<StackStreamItem>();
        var service = Services.GetRequiredService<IApplyStackService>();
        await foreach (var item in service.ApplyAsync(
            stackId,
            Constants.SystemId,
            serviceNames: null,
            pullImages: false,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
            result.Add(item);
        }

        return result;
    }

    private async Task ReconcileCurrentOperationAsync()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        Assert.NotNull(stack);
        Assert.Equal(ResourceControlState.Processing, stack.ControlState);

        stack.ReleaseProcessing(StackReleaseStatus.Unknown);
        var updated = await uow.Stacks.UpdateProcessingAsync(
            stack.Id,
            StackReleaseStatus.Unknown,
            stack.ControlState,
            startedAt: null,
            stack.RowVersion,
            checkRowVersion: true,
            controlTriggeredBy: null,
            TestContext.Current.CancellationToken);
        Assert.True(updated);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkCurrentOperationProcessingAsync()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        Assert.NotNull(stack);
        Assert.True(stack.MarkProcessing(Constants.SystemId));
        stack.PartialUpdate(StackReleaseStatus.Applying);

        var updated = await uow.Stacks.UpdateProcessingAsync(
            stack.Id,
            StackReleaseStatus.Applying,
            stack.ControlState,
            stack.ControlStartedAt,
            stack.RowVersion,
            checkRowVersion: true,
            stack.ControlTriggeredBy,
            TestContext.Current.CancellationToken);
        Assert.True(updated);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task SetCurrentReleaseStatusAsync(StackReleaseStatus status)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        Assert.NotNull(stack);

        await uow.Stacks.UpdateReleaseStatusAsync(
            stack.CurrentStackReleaseId,
            status,
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private static DockerContainer Container(string id)
        => new(
            Name: $"/{id}",
            Image: "nginx:latest",
            Id: id,
            ImageId: "sha256:nginx",
            State: ContainerStateStatus.Running,
            Stack: "recovery-stack");

    private static IReadOnlyDictionary<string, DockerContainer> Containers(
        DockerContainer container)
        => new Dictionary<string, DockerContainer>
        {
            [container.Id] = container
        };

    private static IReadOnlyDictionary<string, DockerContainer> EmptyContainers()
        => new Dictionary<string, DockerContainer>();

    private static async IAsyncEnumerable<StackApplyResult> SuccessfulApplyStream(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        yield return StackApplyResult.StdOut("compose up");
        yield return StackApplyResult.Finished(0);
        await Task.CompletedTask;
    }

    private sealed class EmptyResourceBindingResolver : IResourceBindingResolver
    {
        public Task<Result<ResolvedResourceBindings>> ResolveAsync(
            ResourceBindingScope scope,
            Guid resourceId,
            CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(new ResolvedResourceBindings(
                [],
                [],
                [],
                VariableCount: 0,
                SecretCount: 0)));
    }

    private sealed class TestStackOperationBarrier : IStackOperationBarrier
    {
        private readonly TaskCompletionSource reached =
            new(TaskCreationOptions.RunContinuationsAsynchronously);
        private readonly TaskCompletionSource released =
            new(TaskCreationOptions.RunContinuationsAsynchronously);
        private StackOperationCheckpoint target;
        private int blocked;

        public void BlockAt(StackOperationCheckpoint checkpoint)
        {
            target = checkpoint;
        }

        public async ValueTask WaitAsync(
            StackOperationCheckpoint checkpoint,
            Guid stackId,
            long operationRowVersion,
            CancellationToken cancellationToken)
        {
            if (checkpoint != target || Interlocked.CompareExchange(ref blocked, 1, 0) != 0)
                return;

            reached.TrySetResult();
            await released.Task.WaitAsync(cancellationToken);
        }

        public Task WaitUntilReachedAsync(CancellationToken cancellationToken)
            => reached.Task.WaitAsync(TimeSpan.FromSeconds(10), cancellationToken);

        public void Release()
        {
            released.TrySetResult();
        }
    }
}
