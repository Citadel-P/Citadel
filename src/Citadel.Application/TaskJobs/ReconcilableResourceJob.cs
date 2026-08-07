using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Automation;
using Domain.Entities.Backups;
using Domain.Entities.Builds;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Domain.Entities.Stacks;

namespace Application.TaskJobs;

internal class ReconcilableResourceJob(
    IDbWorkQueue dbWorkQueue,
    IStackStreamManager stackHub,
    INotificationQueue notifQueue,
    IServiceScopeFactory scopeFactory,
    IDeploymentStreamManager deploymentHub,
    IBackupRepositoryStreamManager backupRepositoryStreamManager,
    IBackupPolicyStreamManager backupPolicyStreamManager,
    IAutomationActionStreamManager automationActionStreamManager,
    IBuildProjectStreamManager buildProjectStreamManager,
    IBuildAgentPoolStreamManager buildAgentPoolStreamManager,
    IBuildRunStreamManager buildRunStreamManager,
    IGitRepositoryStreamManager gitRepositoryStreamManager,
    IDockerDaemonStreamManager dockerDaemonHub,
    IDelayWithJitterService delayWithJitterService,
    IContainerEventBroadcaster containerEventBroadcaster,
    ISwarmReconciliationCoordinator swarmReconciliationCoordinator,
    ILogger<ReconcilableResourceJob> logger) : BackgroundService
{
    private static readonly TimeSpan SyncInterval = TimeSpan.FromMinutes(5);
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await delayWithJitterService.DelayWithJitterForAsync(RunPeriodicJanitor, cancellationToken: stoppingToken);
    }

    private async Task RunPeriodicJanitor(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await using (var scope = scopeFactory.CreateAsyncScope())
                {
                    var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

                    // Deployments
                    var stuckDeployments = await uow.Deployments.GetStuckDeploymentsAsync(cancellationToken: cancellationToken);
                    if (stuckDeployments.Any())
                    {
                        var workItem = new StuckDeploymentsSyncWorkItem(deploymentHub, notifQueue, stuckDeployments);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Stacks
                    var stuckStacks = await uow.Stacks.GetStuckStacksAsync(cancellationToken: cancellationToken);
                    if (stuckStacks.Any())
                    {
                        var workItem = new StuckStacksSyncWorkItem(notifQueue, stackHub, stuckStacks);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Git repositories
                    var stuckGitRepositories = (await uow.GitRepositories.GetStuckRepositoriesAsync(cancellationToken: cancellationToken)).ToArray();
                    if (stuckGitRepositories.Length > 0)
                    {
                        var workItem = new StuckGitRepositoriesSyncWorkItem(
                            notifQueue,
                            gitRepositoryStreamManager,
                            stuckGitRepositories);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Containers
                    var stuckContainers = await uow.Containers.GetStuckContainersAsync(cancellationToken: cancellationToken);
                    if (stuckContainers.Any())
                    {
                        var workItem = new StuckContainersSyncWorkItem(notifQueue, dockerDaemonHub, containerEventBroadcaster, stuckContainers);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Images
                    var stuckImages = await uow.Images.GetStuckImagesAsync(cancellationToken: cancellationToken);
                    if (stuckImages.Any())
                    {
                        var workItem = new StuckImagesSyncWorkItem(notifQueue, dockerDaemonHub, stuckImages);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Backup repositories
                    var stuckBackupRepositories = (await uow.BackupRepositories.GetStuckRepositoriesAsync(cancellationToken: cancellationToken)).ToArray();
                    if (stuckBackupRepositories.Length > 0)
                    {
                        var workItem = new StuckBackupRepositoriesSyncWorkItem(
                            notifQueue,
                            backupRepositoryStreamManager,
                            stuckBackupRepositories);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Backup policies
                    var stuckBackupPolicies = (await uow.BackupPolicies.GetStuckPoliciesAsync(cancellationToken: cancellationToken)).ToArray();
                    if (stuckBackupPolicies.Length > 0)
                    {
                        var workItem = new StuckBackupPoliciesSyncWorkItem(
                            notifQueue,
                            backupPolicyStreamManager,
                            stuckBackupPolicies);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Automation actions
                    var stuckAutomationActions = (await uow.AutomationActions.GetStuckActionsAsync(cancellationToken: cancellationToken)).ToArray();
                    if (stuckAutomationActions.Length > 0)
                    {
                        var workItem = new StuckAutomationActionsSyncWorkItem(
                            notifQueue,
                            automationActionStreamManager,
                            stuckAutomationActions);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Build projects
                    var stuckBuildProjects = (await uow.BuildProjects.GetStuckProjectsAsync(cancellationToken: cancellationToken)).ToArray();
                    if (stuckBuildProjects.Length > 0)
                    {
                        var workItem = new StuckBuildProjectsSyncWorkItem(
                            notifQueue,
                            buildProjectStreamManager,
                            buildRunStreamManager,
                            stuckBuildProjects);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Build agent pools
                    var stuckBuildAgentPools = (await uow.BuildAgentPools.GetStuckPoolsAsync(cancellationToken: cancellationToken)).ToArray();
                    if (stuckBuildAgentPools.Length > 0)
                    {
                        var workItem = new StuckBuildAgentPoolsSyncWorkItem(
                            notifQueue,
                            buildAgentPoolStreamManager,
                            stuckBuildAgentPools);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }
                }

                await ReconcileStuckSwarmServicesAsync(cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error during periodic {JobName} synchronization.", nameof(ReconcilableResourceJob));
            }

            await Task.Delay(SyncInterval, cancellationToken);
        }
    }

    internal async Task ReconcileStuckSwarmServicesAsync(CancellationToken cancellationToken)
    {
        Guid[] platformIds;
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            platformIds = (await uow.SwarmServices
                    .GetStuckOperationsAsync(cancellationToken: cancellationToken))
                .Select(static service => service.PlatformId)
                .Distinct()
                .ToArray();
        }

        foreach (var platformId in platformIds)
        {
            var result = await swarmReconciliationCoordinator.RefreshAsync(platformId, cancellationToken);
            if (result.IsFailure(out var error))
                logger.LogWarning(
                    "Stuck Swarm Service recovery failed for platform {PlatformId}: {Error}",
                    platformId,
                    error?.Message);
        }
    }

    internal sealed class StuckDeploymentsSyncWorkItem(
        IDeploymentStreamManager deploymentHub,
        INotificationQueue notificationQueue,
        IEnumerable<Deployment> deployments)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<Deployment>();

            foreach (var deployment in deployments)
            {
                deployment.ReleaseProcessing(DeploymentStatus.Unknown);
                var row = await uow.Deployments.UpdateProcessingAsync(
                                    id: deployment.Id,
                                    status: deployment.Status,
                                    state: deployment.ControlState,
                                    startedAt: null,
                                    rowVersion: deployment.RowVersion,
                                    checkRowVersion: true,
                                    controlTriggeredBy: null,
                                    cancellationToken);
                if (row > 0)
                {
                    successfullyUpdated.Add(deployment);
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var deployment in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentHub, deployment), cancellationToken);
            }
        }
    }

    internal sealed class StuckStacksSyncWorkItem(
        INotificationQueue notificationQueue,
        IStackStreamManager stackHub,
        IEnumerable<Stack> stacks)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<Stack>();

            foreach (var stack in stacks)
            {
                stack.ReleaseProcessing(StackReleaseStatus.Unknown);
                var row = await uow.Stacks.UpdateProcessingAsync(
                                    id: stack.Id,
                                    status: stack.CurrentStackRelease?.Status ?? StackReleaseStatus.Unknown,
                                    state: stack.ControlState,
                                    startedAt: null,
                                    rowVersion: stack.RowVersion,
                                    checkRowVersion: true,
                                    controlTriggeredBy: null,
                                    cancellationToken);
                if (row)
                {
                    successfullyUpdated.Add(stack);
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var stack in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, stack), cancellationToken);
            }
        }
    }

    internal sealed class StuckContainersSyncWorkItem(
        INotificationQueue notificationQueue,
        IDockerDaemonStreamManager dockerDaemonHub,
        IContainerEventBroadcaster containerEventBroadcaster,
        IEnumerable<Container> containers)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<Container>();

            foreach (var container in containers)
            {
                container.ReleaseProcessing();
                var row = await uow.Containers.UpdateProcessingAsync(
                                    id: container.Id,
                                    state: container.ControlState,
                                    startedAt: null,
                                    rowVersion: container.RowVersion,
                                    checkRowVersion: true,
                                    controlTriggeredBy: null,
                                    cancellationToken);
                if (row > 0)
                {
                    successfullyUpdated.Add(container);
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var container in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(new ContainerNotificationWorkItem(container,
                new DaemonContainerEventInfo("processing", container.DockerContainerId, null), dockerDaemonHub, containerEventBroadcaster), cancellationToken);
            }
        }
    }

    internal sealed class StuckGitRepositoriesSyncWorkItem(
        INotificationQueue notificationQueue,
        IGitRepositoryStreamManager streamManager,
        IEnumerable<GitRepository> repositories)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<GitRepository>();

            foreach (var repository in repositories)
            {
                var rowVersion = repository.RowVersion;
                repository.ReleaseProcessing(GitReposStatus.Degraded);
                var row = await uow.GitRepositories.UpdateProcessingAsync(
                    id: repository.Id,
                    status: repository.Status,
                    state: repository.ControlState,
                    startedAt: repository.ControlStartedAt,
                    rowVersion: rowVersion,
                    checkRowVersion: true,
                    controlTriggeredBy: repository.ControlTriggeredBy,
                    cancellationToken);

                if (row > 0)
                    successfullyUpdated.Add(repository);
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var repository in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(
                    new GitRepoNotificationWorkItem(streamManager, repository),
                    cancellationToken);
            }
        }
    }

    internal sealed class StuckImagesSyncWorkItem(
        INotificationQueue notificationQueue,
        IDockerDaemonStreamManager dockerDaemonHub,
        IEnumerable<Image> images)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<Image>();

            foreach (var container in images)
            {
                container.ReleaseProcessing();
                var row = await uow.Images.UpdateProcessingAsync(
                                    id: container.Id,
                                    state: container.ControlState,
                                    startedAt: null,
                                    rowVersion: container.RowVersion,
                                    checkRowVersion: true,
                                    cancellationToken);
                if (row > 0)
                {
                    successfullyUpdated.Add(container);
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var image in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(new ImageNotificationWorkItem(dockerDaemonHub, image, "update"), cancellationToken);
            }
        }
    }

    internal sealed class StuckBackupRepositoriesSyncWorkItem(
        INotificationQueue notificationQueue,
        IBackupRepositoryStreamManager streamManager,
        IEnumerable<BackupRepository> repositories) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<BackupRepository>();

            foreach (var repository in repositories)
            {
                var rowVersion = repository.RowVersion;
                repository.MarkIdle(repository.CurrentRunId, DateTimeOffset.UtcNow);
                var row = await uow.BackupRepositories.UpdateProcessingAsync(
                    id: repository.Id,
                    state: repository.ControlState,
                    startedAt: repository.ControlStartedAt,
                    rowVersion: rowVersion,
                    checkRowVersion: true,
                    currentRunId: repository.CurrentRunId,
                    cancellationToken);

                if (row > 0)
                    successfullyUpdated.Add(repository);
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var repository in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(
                    new BackupRepositoryNotificationWorkItem(streamManager, repository),
                    cancellationToken);
            }
        }
    }

    internal sealed class StuckBackupPoliciesSyncWorkItem(
        INotificationQueue notificationQueue,
        IBackupPolicyStreamManager streamManager,
        IEnumerable<BackupPolicy> policies) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<BackupPolicy>();

            foreach (var policy in policies)
            {
                var rowVersion = policy.RowVersion;
                policy.MarkIdle(policy.CurrentRunId, DateTimeOffset.UtcNow);
                var row = await uow.BackupPolicies.UpdateProcessingAsync(
                    id: policy.Id,
                    state: policy.ControlState,
                    startedAt: policy.ControlStartedAt,
                    rowVersion: rowVersion,
                    checkRowVersion: true,
                    currentRunId: policy.CurrentRunId,
                    cancellationToken);

                if (row > 0)
                    successfullyUpdated.Add(policy);
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var policy in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(
                    new BackupPolicyNotificationWorkItem(streamManager, policy),
                    cancellationToken);
            }
        }
    }

    internal sealed class StuckAutomationActionsSyncWorkItem(
        INotificationQueue notificationQueue,
        IAutomationActionStreamManager streamManager,
        IEnumerable<AutomationAction> actions) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<AutomationAction>();

            foreach (var action in actions)
            {
                var rowVersion = action.RowVersion;
                action.MarkIdle();
                var row = await uow.AutomationActions.UpdateProcessingAsync(
                    id: action.Id,
                    state: action.ControlState,
                    startedAt: action.ControlStartedAt,
                    rowVersion: rowVersion,
                    checkRowVersion: true,
                    currentRunId: action.CurrentRunId,
                    cancellationToken);

                if (row > 0)
                    successfullyUpdated.Add(action);
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var action in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(
                    new AutomationActionNotificationWorkItem(streamManager, action),
                    cancellationToken);
            }
        }
    }

    internal sealed class StuckBuildProjectsSyncWorkItem(
        INotificationQueue notificationQueue,
        IBuildProjectStreamManager streamManager,
        IBuildRunStreamManager buildRunStreamManager,
        IEnumerable<BuildProject> projects) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<BuildProject>();
            var interruptedRuns = new List<BuildRun>();

            foreach (var project in projects)
            {
                var rowVersion = project.RowVersion;
                var currentRunId = project.CurrentRunId;
                var now = DateTimeOffset.UtcNow;
                project.MarkIdle(currentRunId ?? Guid.Empty, now);
                var row = await uow.BuildProjects.UpdateProcessingAsync(
                    id: project.Id,
                    state: project.ControlState,
                    startedAt: project.ControlStartedAt,
                    rowVersion: rowVersion,
                    checkRowVersion: true,
                    currentRunId: project.CurrentRunId,
                    cancellationToken);

                if (row > 0)
                {
                    successfullyUpdated.Add(project);

                    if (currentRunId.HasValue)
                    {
                        var interruptedRun = await uow.BuildRuns.InterruptQueuedOrRunningAsync(
                            currentRunId.Value,
                            now,
                            "Build run was interrupted after the project stayed in processing state for too long.",
                            cancellationToken);
                        if (interruptedRun is not null)
                            interruptedRuns.Add(interruptedRun);
                    }
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var run in interruptedRuns)
            {
                await notificationQueue.EnqueueAsync(
                    new BuildRunNotificationWorkItem(buildRunStreamManager, run),
                    cancellationToken);
            }

            foreach (var project in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(
                    new BuildProjectNotificationWorkItem(streamManager, project),
                    cancellationToken);
            }
        }
    }

    internal sealed class StuckBuildAgentPoolsSyncWorkItem(
        INotificationQueue notificationQueue,
        IBuildAgentPoolStreamManager streamManager,
        IEnumerable<BuildAgentPool> pools) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<BuildAgentPool>();

            foreach (var pool in pools)
            {
                var rowVersion = pool.RowVersion;
                pool.MarkIdle(DateTimeOffset.UtcNow);
                var row = await uow.BuildAgentPools.UpdateProcessingAsync(
                    id: pool.Id,
                    state: pool.ControlState,
                    startedAt: pool.ControlStartedAt,
                    rowVersion: rowVersion,
                    checkRowVersion: true,
                    controlTriggeredBy: pool.ControlTriggeredBy,
                    cancellationToken);

                if (row > 0)
                    successfullyUpdated.Add(pool);
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var pool in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(
                    new BuildAgentPoolNotificationWorkItem(streamManager, pool),
                    cancellationToken);
            }
        }
    }
}
