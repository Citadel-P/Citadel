using Application.Services;
using Application.Services.Alerts;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using System.Threading.Channels;
using Tests.Common;

namespace Tests.Unit.Application.TaskJobs;

public sealed class StackWebhookDeployJobTests
{
    [Fact]
    public async Task PersistedFailedDeploy_RaisesAlertAndDeletesQueueItem()
    {
        var (repository, stack, spec) = CreateStack();
        var item = CreateQueueItem(stack, repository, spec) with
        {
            Status = StackWebhookDeployQueueStatus.Processing,
            Attempts = 3
        };
        var completed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var queue = CreateQueue(item, completed);
        var unitOfWork = CreateUnitOfWork(stack, repository, queue.Object);

        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        var applyStackService = new Mock<IApplyStackService>();
        applyStackService
            .Setup(service => service.ApplyAsync(
                stack.Id,
                Constants.SystemId,
                null,
                true,
                false,
                true,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Returns(FailedStream("compose failed"));

        var alertService = new Mock<IAlertService>();
        alertService
            .Setup(service => service.ProcessAsync(
                AlertType.WebhookStackGitDeployFailed,
                It.Is<AlertEvaluationContext>(context =>
                    context.StackGitWebhookDeployFailures != null
                    && context.StackGitWebhookDeployFailures.Single().Id == stack.Id
                    && context.StackGitWebhookDeployFailures.Single().Reason == "compose failed"),
                It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var job = CreateJob(
            services,
            applyStackService.Object,
            alertService.Object);

        await job.StartAsync(TestContext.Current.CancellationToken);
        try
        {
            await completed.Task.WaitAsync(
                TimeSpan.FromSeconds(2),
                TestContext.Current.CancellationToken);
        }
        finally
        {
            await job.StopAsync(TestContext.Current.CancellationToken);
        }

        queue.Verify(x => x.RequeueInterruptedAsync(
            It.IsAny<DateTime>(),
            It.IsAny<CancellationToken>()), Times.Once);
        queue.Verify(x => x.DeleteAsync(item.Id, It.IsAny<CancellationToken>()), Times.Once);
        alertService.VerifyAll();
    }

    [Fact]
    public async Task PersistedDeploy_WhenStackConfigurationChanged_IsDiscardedWithoutApplying()
    {
        var (repository, stack, originalSpec) = CreateStack();
        var item = CreateQueueItem(stack, repository, originalSpec) with
        {
            Status = StackWebhookDeployQueueStatus.Processing,
            Attempts = 1
        };
        var changedSpec = originalSpec with { WorkingDirectory = "changed" };
        Assert.True(stack.UpdateCurrentStackReleaseDefinition(
            stack.CurrentStackRelease!.PlatformId,
            changedSpec));

        var completed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var queue = CreateQueue(item, completed);
        var unitOfWork = CreateUnitOfWork(stack, repository, queue.Object);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var applyStackService = new Mock<IApplyStackService>();

        var job = CreateJob(
            services,
            applyStackService.Object,
            Mock.Of<IAlertService>());

        await job.StartAsync(TestContext.Current.CancellationToken);
        try
        {
            await completed.Task.WaitAsync(
                TimeSpan.FromSeconds(2),
                TestContext.Current.CancellationToken);
        }
        finally
        {
            await job.StopAsync(TestContext.Current.CancellationToken);
        }

        applyStackService.Verify(
            x => x.ApplyAsync(
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
        queue.Verify(x => x.DeleteAsync(item.Id, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task PersistedDeploy_WithProgressMessages_CompletesWithoutFailureAlert()
    {
        var (repository, stack, spec) = CreateStack();
        var item = CreateQueueItem(stack, repository, spec) with
        {
            Status = StackWebhookDeployQueueStatus.Processing,
            Attempts = 1
        };
        var completed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var queue = CreateQueue(item, completed);
        var unitOfWork = CreateUnitOfWork(stack, repository, queue.Object);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var applyStackService = new Mock<IApplyStackService>();
        applyStackService
            .Setup(x => x.ApplyAsync(
                stack.Id,
                Constants.SystemId,
                null,
                true,
                false,
                true,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Returns(SuccessfulStream());
        var alertService = new Mock<IAlertService>();

        var job = CreateJob(services, applyStackService.Object, alertService.Object);
        await job.StartAsync(TestContext.Current.CancellationToken);
        try
        {
            await completed.Task.WaitAsync(
                TimeSpan.FromSeconds(2),
                TestContext.Current.CancellationToken);
        }
        finally
        {
            await job.StopAsync(TestContext.Current.CancellationToken);
        }

        queue.Verify(x => x.DeleteAsync(item.Id, It.IsAny<CancellationToken>()), Times.Once);
        queue.Verify(
            x => x.RetryAsync(
                It.IsAny<Guid>(),
                It.IsAny<string>(),
                It.IsAny<DateTime>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        alertService.Verify(
            x => x.ProcessAsync(
                It.IsAny<AlertType>(),
                It.IsAny<AlertEvaluationContext>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task TransientStartupRecoveryFailure_RetriesWithoutTerminatingWorker()
    {
        var recovered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var recoveryAttempts = 0;
        var queueRepository = new Mock<IStackWebhookDeployQueueRepository>();
        queueRepository
            .Setup(x => x.RequeueInterruptedAsync(
                It.IsAny<DateTime>(),
                It.IsAny<CancellationToken>()))
            .Returns(() =>
            {
                if (Interlocked.Increment(ref recoveryAttempts) == 1)
                    throw new InvalidOperationException("database unavailable");

                recovered.TrySetResult();
                return Task.FromResult(0);
            });
        queueRepository
            .Setup(x => x.GetReadyIdsAsync(
                It.IsAny<int>(),
                It.IsAny<DateTime>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.StackWebhookDeployQueue).Returns(queueRepository.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var channel = Channel.CreateBounded<StackWebhookDeploySignal>(4);
        var job = CreateJob(
            services,
            Mock.Of<IApplyStackService>(),
            Mock.Of<IAlertService>(),
            channel.Reader);

        await job.StartAsync(TestContext.Current.CancellationToken);
        try
        {
            await channel.Writer.WriteAsync(
                new StackWebhookDeploySignal(Guid.CreateVersion7()),
                TestContext.Current.CancellationToken);
            await recovered.Task.WaitAsync(
                TimeSpan.FromSeconds(2),
                TestContext.Current.CancellationToken);
        }
        finally
        {
            await job.StopAsync(TestContext.Current.CancellationToken);
        }

        Assert.Equal(2, recoveryAttempts);
    }

    private static StackWebhookDeployJob CreateJob(
        ServiceProvider services,
        IApplyStackService applyStackService,
        IAlertService alertService,
        ChannelReader<StackWebhookDeploySignal>? reader = null)
    {
        var channel = Channel.CreateBounded<StackWebhookDeploySignal>(4);
        var scopeFactory = services.GetRequiredService<IServiceScopeFactory>();
        var queue = new StackWebhookDeployQueueService(
            scopeFactory,
            TimeProvider.System);
        var processor = new StackWebhookDeployProcessor(
            queue,
            scopeFactory,
            applyStackService,
            alertService,
            new PermissiveLicenseEntitlementService(),
            TimeProvider.System,
            NullLogger<StackWebhookDeployProcessor>.Instance);
        return new StackWebhookDeployJob(
            reader ?? channel.Reader,
            queue,
            processor,
            TimeProvider.System,
            NullLogger<StackWebhookDeployJob>.Instance);
    }

    private static Mock<IStackWebhookDeployQueueRepository> CreateQueue(
        StackWebhookDeployQueueItem item,
        TaskCompletionSource completed)
    {
        var readyRead = 0;
        var queue = new Mock<IStackWebhookDeployQueueRepository>();
        queue
            .Setup(x => x.RequeueInterruptedAsync(
                It.IsAny<DateTime>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        queue
            .Setup(x => x.GetReadyIdsAsync(
                It.IsAny<int>(),
                It.IsAny<DateTime>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => Interlocked.Increment(ref readyRead) == 1 ? [item.Id] : []);
        queue
            .Setup(x => x.TryClaimAsync(
                item.Id,
                It.IsAny<DateTime>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(item);
        queue
            .Setup(x => x.DeleteAsync(item.Id, It.IsAny<CancellationToken>()))
            .Callback(() => completed.TrySetResult())
            .ReturnsAsync(1);
        return queue;
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        Stack stack,
        GitRepository repository,
        IStackWebhookDeployQueueRepository queue)
    {
        var stackRepository = new Mock<IStackRepository>();
        stackRepository
            .Setup(stacks => stacks.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        var gitRepository = new Mock<IGitReposRepository>();
        gitRepository
            .Setup(repositories => repositories.GetAsync(repository.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.Stacks).Returns(stackRepository.Object);
        unitOfWork.SetupGet(x => x.GitRepositories).Returns(gitRepository.Object);
        unitOfWork.SetupGet(x => x.StackWebhookDeployQueue).Returns(queue);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static (GitRepository Repository, Stack Stack, GitStack Spec) CreateStack()
    {
        var repository = new GitRepository(
            name: "repo",
            description: null,
            url: "https://example.test/repo.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);
        var spec = new GitStack(
            repository.Id,
            "main",
            CommitSha: null,
            StackUpdateBehavior.StackAutoDeploy,
            Webhook: new StackWebhookConfig(Enabled: true));
        var stack = Stack.Create(
            name: "stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: Guid.CreateVersion7(),
            spec: spec);
        return (repository, stack, spec);
    }

    private static StackWebhookDeployQueueItem CreateQueueItem(
        Stack stack,
        GitRepository repository,
        GitStack spec)
        => StackWebhookDeployQueueItem.Create(
            stack.Id,
            repository.Id,
            stack.CurrentStackReleaseId,
            spec.Branch,
            StackWebhookDeployFingerprint.Compute(spec),
            "0123456789abcdef",
            DateTime.UtcNow);

    private static async IAsyncEnumerable<StackStreamItem> FailedStream(string message)
    {
        await Task.CompletedTask;
        yield return StackStreamItem.FromStdErr(message, 1);
    }

    private static async IAsyncEnumerable<StackStreamItem> SuccessfulStream()
    {
        await Task.CompletedTask;
        yield return StackStreamItem.SystemMessage("Resolving stack variables...", 0);
        yield return StackStreamItem.FromStdOut("Container started");
        yield return StackStreamItem.Finished(0);
    }
}
