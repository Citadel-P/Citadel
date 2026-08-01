using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Automation;
using Domain.Entities.Backups;
using Domain.Entities.Deployments;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ReconcilableResourceJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private const int StaleResourceAgeSeconds = 3_700;
    private readonly Mock<IDeploymentStreamManager> streamManagerMock = new();
    private readonly Mock<IStackStreamManager> stackStreamManagerMock = new();
    private readonly Mock<IDockerDaemonStreamManager> dockerDaemonStreamManagerMock = new();
    private readonly Mock<IContainerEventBroadcaster> containerEventBroadcasterMock = new();
    private readonly Mock<IBackupRepositoryStreamManager> backupRepositoryStreamManagerMock = new();
    private readonly Mock<IBackupPolicyStreamManager> backupPolicyStreamManagerMock = new();
    private readonly Mock<IAutomationActionStreamManager> automationActionStreamManagerMock = new();
    private readonly Mock<IDelayWithJitterService> _delayWithJitter = new();
    private readonly Mock<INotificationQueue> notificationMock = new();
    private Guid platformId;
    private Guid backupRepositoryId;
    private Guid backupPolicyId;
    private Guid automationActionId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IDelayWithJitterService>();

        services.AddSingleton(streamManagerMock.Object);
        services.AddSingleton(stackStreamManagerMock.Object);
        services.AddSingleton(dockerDaemonStreamManagerMock.Object);
        services.AddSingleton(containerEventBroadcasterMock.Object);
        services.AddSingleton(backupRepositoryStreamManagerMock.Object);
        services.AddSingleton(backupPolicyStreamManagerMock.Object);
        services.AddSingleton(automationActionStreamManagerMock.Object);
        services.AddSingleton(notificationMock.Object);
        services.AddSingleton(_ => _delayWithJitter.Object);

        _delayWithJitter
         .Setup(x => x.DelayWithJitterForAsync(It.IsAny<Func<CancellationToken, Task>>(),
                                               It.IsAny<TimeSpan>(),
                                               It.IsAny<CancellationToken>()))
         .Returns<Func<CancellationToken, Task>, TimeSpan, CancellationToken>((_, _, ct) => Task.Delay(Timeout.InfiniteTimeSpan, ct));
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        platformId = platform.Id;

        var deployment = new Deployment
        (
            name: "Test Deployment",
            description: "A deployment for testing",
            platformId: platform.Id,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec
            (
                UpdateBehavior: UpdateBehavior.AutoDeploy,
                Image: new ExternalImage
                (
                    RegistryId: Constants.DefaultRegistryId,
                    ImageTag: "nginx:latest"
                ),
                Ports: new List<string> { "80:80" }
            )
        );
        var stack = Stack.Create
        (
            name: "Test Stack",
            description: "A stack for testing",
            platformId: platform.Id,
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            spec: new ManualStack
            (
                ComposeFile: "",
                UpdateBehavior: StackUpdateBehavior.Notify,
                RegistryId: Constants.DefaultRegistryId
            )
        );
        var container = new Container(
            platformId: platformId,
            dockerContainerId: "container-123",
            dockerImageId: "image",
            name: "deployment-container",
            created: 999999,
            state: ContainerStateStatus.Running,
            deploymentId: deployment.Id
        );
        var image = new Image(
            platformId: platformId,
            dockerImageId: "image",
            name: "deployment-image",
            tags: new List<string> { "nginx:latest" },
            containers: 1,
            size: 123456,
            createdAt: DateTime.UtcNow
        );
        var passwordSecret = new SecretDefinition("RESTIC_PASSWORD_RECONCILABLE_RESOURCE_JOB", SecretProviderType.InternalEncrypted);
        var backupRepository = new BackupRepository(
            name: "Reconcilable repository",
            description: null,
            spec: new FileSystemBackupRepositorySpec(BackupExecutionLocation.Core, null, "backups/reconcilable"),
            passwordSecretId: passwordSecret.Id,
            createdByActorId: Constants.SystemId);
        var backupPolicy = new BackupPolicy(
            name: "Reconcilable policy",
            description: null,
            source: new CitadelSystemBackupSource(),
            backupRepositoryId: backupRepository.Id,
            enabled: true,
            cron: null,
            timeZone: null,
            webhook: null,
            keepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            timeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            alertOnFailure: true,
            runAsActorId: Constants.SystemId,
            createdByActorId: Constants.SystemId);
        var automationAction = new AutomationAction(
            name: "Reconcilable action",
            description: null,
            code: "console.log('ok');",
            defaultArgsJson: "{}",
            enabled: true,
            scheduleEnabled: false,
            scheduleCron: null,
            scheduleTimeZone: "UTC",
            webhook: null,
            timeoutSeconds: 300,
            alertOnFailure: true,
            runAsActorId: Constants.SystemId,
            createdByActorId: Constants.SystemId);

        backupRepositoryId = backupRepository.Id;
        backupPolicyId = backupPolicy.Id;
        automationActionId = automationAction.Id;

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await uow.Images.AddOrUpdateAsync(image, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.SecretDefinitions.AddAsync(
            passwordSecret,
            new InternalSecretValue(passwordSecret.Id, "encrypted"),
            TestContext.Current.CancellationToken);
        await uow.BackupRepositories.AddAsync(backupRepository, TestContext.Current.CancellationToken);
        await uow.BackupPolicies.AddAsync(backupPolicy, TestContext.Current.CancellationToken);
        await uow.AutomationActions.AddAsync(automationAction, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Deployment()
    {
        // Arrange
        await MarkDeploymentAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - StaleResourceAgeSeconds);
        await RunJanitorOnceAsync();

        var deployment = await WaitForAsync(
            async uow => (await uow.Deployments.GetInfoAsync(TestContext.Current.CancellationToken)).First(),
            deployment => deployment.ControlState == ResourceControlState.Idle);
        Assert.Equal(ResourceControlState.Idle, deployment.ControlState);
        Assert.Null(deployment.ControlTriggeredBy);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<DeploymentNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Stack()
    {
        // Arrange
        await MarkStackAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - StaleResourceAgeSeconds);
        await RunJanitorOnceAsync();

        var stack = await WaitForAsync(
            async uow => (await uow.Stacks.GetAllAsync(TestContext.Current.CancellationToken)).First(),
            stack => stack.ControlState == ResourceControlState.Idle);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
        Assert.Null(stack.ControlTriggeredBy);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<StackNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Container()
    {
        // Arrange
        await MarkContainerAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - 90);
        await RunJanitorOnceAsync();

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var container = (await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();
        Assert.Equal(ResourceControlState.Idle, container.ControlState);
        Assert.Null(container.ControlTriggeredBy);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ContainerNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_Image()
    {
        // Arrange
        await MarkImageAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - 90);
        await RunJanitorOnceAsync();

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var image = (await uow.Images.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();
        Assert.Equal(ResourceControlState.Idle, image.ControlState);

        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ImageNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Once);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_BackupRepository()
    {
        await MarkBackupRepositoryAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - StaleResourceAgeSeconds);
        await RunJanitorOnceAsync();

        var repository = await WaitForAsync(
            uow => uow.BackupRepositories.GetAsync(backupRepositoryId, TestContext.Current.CancellationToken),
            repository => repository?.ControlState == ResourceControlState.Idle);

        Assert.NotNull(repository);
        Assert.Equal(ResourceControlState.Idle, repository.ControlState);
        Assert.Null(repository.CurrentRunId);
        Assert.Null(repository.ControlStartedAt);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_BackupPolicy()
    {
        await MarkBackupPolicyAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - StaleResourceAgeSeconds);
        await RunJanitorOnceAsync();

        var policy = await WaitForAsync(
            uow => uow.BackupPolicies.GetAsync(backupPolicyId, TestContext.Current.CancellationToken),
            policy => policy?.ControlState == ResourceControlState.Idle);

        Assert.NotNull(policy);
        Assert.Equal(ResourceControlState.Idle, policy.ControlState);
        Assert.Null(policy.CurrentRunId);
        Assert.Null(policy.ControlStartedAt);
    }

    [Fact]
    public async Task RunPeriodicJanitor_Clean_Stuck_AutomationAction()
    {
        await MarkAutomationActionAsync(ResourceControlState.Processing, DateTimeOffset.UtcNow.ToUnixTimeSeconds() - 90);
        await RunJanitorOnceAsync();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(automationActionId, TestContext.Current.CancellationToken);

        Assert.NotNull(action);
        Assert.Equal(ResourceControlState.Idle, action.ControlState);
        Assert.Null(action.CurrentRunId);
        Assert.Null(action.ControlStartedAt);
    }

    [Fact]
    public async Task RunPeriodicJanitor_DoesNotEnqueue_WhenNoStuckDeployments()
    {
        // Arrange
        await MarkDeploymentAsync(ResourceControlState.Idle, null);
        await RunJanitorOnceAsync();

        // Assert
        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<DeploymentNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Never);
    }

    [Fact]
    public async Task RunPeriodicJanitor_DoesNotEnqueue_WhenNoStuckContainers()
    {
        // Arrange
        await MarkContainerAsync(ResourceControlState.Idle, null);
        await RunJanitorOnceAsync();

        // Assert
        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ContainerNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Never);
    }

    [Fact]
    public async Task RunPeriodicJanitor_DoesNotEnqueue_WhenNoStuckImages()
    {
        // Arrange
        await MarkImageAsync(ResourceControlState.Idle, null);
        await RunJanitorOnceAsync();

        // Assert
        notificationMock.Verify(
           nq => nq.EnqueueAsync(It.IsAny<ImageNotificationWorkItem>(), It.IsAny<CancellationToken>()),
           Times.Never);
    }

    private async Task RunJanitorOnceAsync()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cancellationToken = TestContext.Current.CancellationToken;

        var stuckDeployments = (await uow.Deployments.GetStuckDeploymentsAsync(cancellationToken: cancellationToken)).ToArray();
        if (stuckDeployments.Length > 0)
        {
            await new ReconcilableResourceJob.StuckDeploymentsSyncWorkItem(
                streamManagerMock.Object,
                notificationMock.Object,
                stuckDeployments).ExecuteAsync(uow, cancellationToken);
        }

        var stuckStacks = (await uow.Stacks.GetStuckStacksAsync(cancellationToken: cancellationToken)).ToArray();
        if (stuckStacks.Length > 0)
        {
            await new ReconcilableResourceJob.StuckStacksSyncWorkItem(
                notificationMock.Object,
                stackStreamManagerMock.Object,
                stuckStacks).ExecuteAsync(uow, cancellationToken);
        }

        var stuckContainers = (await uow.Containers.GetStuckContainersAsync(cancellationToken: cancellationToken)).ToArray();
        if (stuckContainers.Length > 0)
        {
            await new ReconcilableResourceJob.StuckContainersSyncWorkItem(
                notificationMock.Object,
                dockerDaemonStreamManagerMock.Object,
                containerEventBroadcasterMock.Object,
                stuckContainers).ExecuteAsync(uow, cancellationToken);
        }

        var stuckImages = (await uow.Images.GetStuckImagesAsync(cancellationToken: cancellationToken)).ToArray();
        if (stuckImages.Length > 0)
        {
            await new ReconcilableResourceJob.StuckImagesSyncWorkItem(
                notificationMock.Object,
                dockerDaemonStreamManagerMock.Object,
                stuckImages).ExecuteAsync(uow, cancellationToken);
        }

        var stuckBackupRepositories = (await uow.BackupRepositories.GetStuckRepositoriesAsync(cancellationToken: cancellationToken)).ToArray();
        if (stuckBackupRepositories.Length > 0)
        {
            await new ReconcilableResourceJob.StuckBackupRepositoriesSyncWorkItem(
                notificationMock.Object,
                backupRepositoryStreamManagerMock.Object,
                stuckBackupRepositories).ExecuteAsync(uow, cancellationToken);
        }

        var stuckBackupPolicies = (await uow.BackupPolicies.GetStuckPoliciesAsync(cancellationToken: cancellationToken)).ToArray();
        if (stuckBackupPolicies.Length > 0)
        {
            await new ReconcilableResourceJob.StuckBackupPoliciesSyncWorkItem(
                notificationMock.Object,
                backupPolicyStreamManagerMock.Object,
                stuckBackupPolicies).ExecuteAsync(uow, cancellationToken);
        }

        var stuckAutomationActions = (await uow.AutomationActions.GetStuckActionsAsync(cancellationToken: cancellationToken)).ToArray();
        if (stuckAutomationActions.Length > 0)
        {
            await new ReconcilableResourceJob.StuckAutomationActionsSyncWorkItem(
                notificationMock.Object,
                automationActionStreamManagerMock.Object,
                stuckAutomationActions).ExecuteAsync(uow, cancellationToken);
        }
    }

    private async Task<T> WaitForAsync<T>(Func<IUnitOfWork, Task<T>> getValue, Func<T, bool> isReady)
    {
        var deadline = DateTimeOffset.UtcNow.AddSeconds(10);
        T value = default!;
        while (DateTimeOffset.UtcNow < deadline)
        {
            TestContext.Current.CancellationToken.ThrowIfCancellationRequested();

            await using var scope = Services.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            value = await getValue(uow);

            if (isReady(value))
                return value;

            await Task.Delay(TimeSpan.FromMilliseconds(100), TestContext.Current.CancellationToken);
        }

        return value;
    }

    private async Task MarkDeploymentAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = (await uow.Deployments.GetInfoAsync(TestContext.Current.CancellationToken)).First();

        await uow.Deployments.UpdateProcessingAsync(
            id: deployment.Id,
            status: deployment.Status,
            state: state,
            startedAt: startedAt,
            rowVersion: deployment.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: deployment.ControlTriggeredBy,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkStackAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = (await uow.Stacks.GetAllAsync(TestContext.Current.CancellationToken)).First();

        await uow.Stacks.UpdateProcessingAsync(
            id: stack.Id,
            status: stack.CurrentStackRelease!.Status,
            state: state,
            startedAt: startedAt,
            rowVersion: stack.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: stack.ControlTriggeredBy,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkContainerAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var container = (await uow.Containers.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();

        await uow.Containers.UpdateProcessingAsync(
            id: container.Id,
            state: state,
            startedAt: startedAt,
            rowVersion: container.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: container.ControlTriggeredBy,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkImageAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var image = (await uow.Images.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken)).First();

        await uow.Images.UpdateProcessingAsync(
            id: image.Id,
            state: state,
            startedAt: startedAt,
            rowVersion: image.RowVersion,
            checkRowVersion: false,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkBackupRepositoryAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var repository = await uow.BackupRepositories.GetAsync(backupRepositoryId, TestContext.Current.CancellationToken);

        await uow.BackupRepositories.UpdateProcessingAsync(
            id: backupRepositoryId,
            state: state,
            startedAt: startedAt,
            rowVersion: repository!.RowVersion,
            checkRowVersion: false,
            currentRunId: state == ResourceControlState.Processing ? Guid.CreateVersion7() : null,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkBackupPolicyAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var policy = await uow.BackupPolicies.GetAsync(backupPolicyId, TestContext.Current.CancellationToken);

        await uow.BackupPolicies.UpdateProcessingAsync(
            id: backupPolicyId,
            state: state,
            startedAt: startedAt,
            rowVersion: policy!.RowVersion,
            checkRowVersion: false,
            currentRunId: state == ResourceControlState.Processing ? Guid.CreateVersion7() : null,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async Task MarkAutomationActionAsync(ResourceControlState state, long? startedAt)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await uow.AutomationActions.GetAsync(automationActionId, TestContext.Current.CancellationToken);

        await uow.AutomationActions.UpdateProcessingAsync(
            id: automationActionId,
            state: state,
            startedAt: startedAt,
            rowVersion: action!.RowVersion,
            checkRowVersion: false,
            currentRunId: state == ResourceControlState.Processing ? Guid.CreateVersion7() : null,
            TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

}
