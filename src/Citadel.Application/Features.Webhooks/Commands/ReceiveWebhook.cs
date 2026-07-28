using Application.Features.Deployments.Notifications;
using Application.Features.Builds.Commands;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using Domain.Entities.Automation;
using Domain.Entities.Backups;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;

namespace Application.Features.Webhooks.Commands;

public sealed record ReceiveWebhook(
    string AuthType,
    string ResourceType,
    Guid ResourceId,
    string Execution,
    IReadOnlyDictionary<string, string[]> Headers,
    byte[] Body) : ICommand<Result<WebhookReceiveResult>>
{
    internal sealed class Validator : AbstractValidator<ReceiveWebhook>
    {
        public Validator()
        {
            RuleFor(x => x.AuthType).NotEmpty();
            RuleFor(x => x.ResourceType).NotEmpty();
            RuleFor(x => x.ResourceId).NotEmpty();
            RuleFor(x => x.Execution).NotEmpty();
            RuleFor(x => x.Headers).NotNull();
            RuleFor(x => x.Body).NotNull();
        }
    }
}

public sealed record WebhookReceiveResult(bool Accepted, string Status, Guid RequestId, string? Reason = null);

internal sealed class ReceiveWebhookHandler(
    IUnitOfWork unitOfWork,
    ChannelWriter<GitRepoSyncRequest> gitSyncWriter,
    ChannelWriter<StackWebhookDeploySignal> stackDeployWriter,
    INotificationQueue notificationQueue,
    IGitRepositoryStreamManager gitRepositoryStreamManager,
    IActivityStreamManager activityStreamManager,
    IAlertService alertService,
    IBuildProjectStreamManager buildProjectStreamManager,
    IBuildRunStreamManager buildRunStreamManager,
    IRepoCacheManager repoCacheManager,
    IGitCliRepository gitCliRepository,
    IAutomationRunQueueService automationRunQueueService,
    ILicenseEntitlementService entitlementService) : ICommandHandler<ReceiveWebhook, Result<WebhookReceiveResult>>
{
    private const int MaxBodyBytes = 1024 * 1024;
    private static readonly TimeSpan GitLabSignedTimestampTolerance = TimeSpan.FromMinutes(5);

    public async ValueTask<Result<WebhookReceiveResult>> Handle(ReceiveWebhook command, CancellationToken cancellationToken)
    {
        var requestId = Guid.CreateVersion7();
        if (command.Body.Length > MaxBodyBytes)
        {
            await RecordActivityAsync(command, requestId, null, null, WebhookDispatchResult.NoOp("Request body too large"), "rejected", "Request body too large", ActivityStatus.Failure, cancellationToken);
            return Result.Failure<WebhookReceiveResult>(new BadRequestError("Request body too large"));
        }

        var target = await ResolveTargetAsync(command, cancellationToken);
        if (target.Error is not null)
        {
            await RecordActivityAsync(command, requestId, target, null, WebhookDispatchResult.NoOp(target.Error.Message), "rejected", target.Error.Message, ActivityStatus.Failure, cancellationToken);
            return Result.Failure<WebhookReceiveResult>(target.Error);
        }

        if (!Authenticate(target.AuthScheme, command.Headers, command.Body, target.Secret))
        {
            await RecordActivityAsync(command, requestId, target, null, WebhookDispatchResult.NoOp("Webhook authentication failed"), "rejected", "Webhook authentication failed", ActivityStatus.Failure, cancellationToken);
            await ProcessWebhookAlertAsync(AlertType.WebhookAuthenticationFailed, command, requestId, target, payload: null, "Webhook authentication failed", cancellationToken);
            return Result.Failure<WebhookReceiveResult>(new UnauthorizedError("Webhook authentication failed"));
        }

        var payload = ParsePayload(target.Provider, command.Headers, command.Body);
        var dispatch = await DispatchAsync(command, target, payload, cancellationToken);
        await RecordActivityAsync(
            command,
            requestId,
            target,
            payload,
            dispatch,
            dispatch.Status,
            dispatch.Reason,
            dispatch.Status.Equals("queued", StringComparison.OrdinalIgnoreCase) ? ActivityStatus.Success : ActivityStatus.Information,
            cancellationToken);

        if (dispatch.Status.Equals("noop", StringComparison.OrdinalIgnoreCase)
            && IsAlertableDispatchNoOp(dispatch.Reason))
        {
            await ProcessWebhookAlertAsync(AlertType.WebhookDispatchFailed, command, requestId, target, payload, dispatch.Reason!, cancellationToken);
        }

        return new WebhookReceiveResult(true, dispatch.Status, requestId, dispatch.Reason);
    }

    private async Task<WebhookTarget> ResolveTargetAsync(ReceiveWebhook command, CancellationToken cancellationToken)
    {
        if (!TryResolveProvider(command.AuthType, out var provider))
            return WebhookTarget.BadRequest("Unsupported webhook auth type.");

        if (command.ResourceType.Equals("repo", StringComparison.OrdinalIgnoreCase)
            && command.Execution.Equals("pull", StringComparison.OrdinalIgnoreCase))
        {
            var repo = await unitOfWork.GitRepositories.GetAsync(command.ResourceId, cancellationToken);
            var webhook = repo?.Webhook;
            if (repo is null || webhook is null || !webhook.Enabled)
                return WebhookTarget.NotFound();

            if (webhook.Provider != provider)
                return WebhookTarget.BadRequest("Webhook auth type does not match repository webhook provider.");

            return new WebhookTarget(
                Provider: webhook.Provider,
                AuthScheme: webhook.AuthScheme,
                Execution: WebhookExecution.RepoPull,
                Secret: webhook.Secret,
                BranchFilter: webhook.BranchFilter ?? repo.DefaultBranch,
                Repository: repo,
                Stack: null,
                GitStack: null,
                Action: null,
                BackupPolicy: null,
                BuildProject: null,
                Error: null);
        }

        if (command.ResourceType.Equals("stack", StringComparison.OrdinalIgnoreCase)
            && command.Execution.Equals("deploy", StringComparison.OrdinalIgnoreCase))
        {
            var stack = await unitOfWork.Stacks.GetAsync(command.ResourceId, cancellationToken);
            if (stack?.CurrentStackRelease?.Spec is not GitStack gitStack)
                return WebhookTarget.NotFound();

            var webhook = gitStack.Webhook;
            if (webhook is null || !webhook.Enabled)
                return WebhookTarget.NotFound();

            if (webhook.Provider != provider)
                return WebhookTarget.BadRequest("Webhook auth type does not match stack webhook provider.");

            return new WebhookTarget(
                Provider: webhook.Provider,
                AuthScheme: webhook.AuthScheme,
                Execution: WebhookExecution.StackDeploy,
                Secret: webhook.Secret,
                BranchFilter: webhook.BranchFilter ?? gitStack.Branch,
                Repository: null,
                Stack: stack,
                GitStack: gitStack,
                Action: null,
                BackupPolicy: null,
                BuildProject: null,
                Error: null);
        }

        if ((command.ResourceType.Equals("automation-action", StringComparison.OrdinalIgnoreCase)
                || command.ResourceType.Equals("action", StringComparison.OrdinalIgnoreCase))
            && command.Execution.Equals("run", StringComparison.OrdinalIgnoreCase))
        {
            var action = await unitOfWork.AutomationActions.GetAsync(command.ResourceId, cancellationToken);
            var webhook = action?.Webhook;
            if (action is null || webhook is null || !webhook.Enabled)
                return WebhookTarget.NotFound();

            if (webhook.Provider != provider)
                return WebhookTarget.BadRequest("Webhook auth type does not match automation action webhook provider.");

            return new WebhookTarget(
                Provider: webhook.Provider,
                AuthScheme: webhook.AuthScheme,
                Execution: WebhookExecution.AutomationActionRun,
                Secret: webhook.Secret,
                BranchFilter: webhook.BranchFilter,
                Repository: null,
                Stack: null,
                GitStack: null,
                Action: action,
                BackupPolicy: null,
                BuildProject: null,
                Error: null);
        }

        if ((command.ResourceType.Equals("backup-policy", StringComparison.OrdinalIgnoreCase)
                || command.ResourceType.Equals("backupPolicy", StringComparison.OrdinalIgnoreCase))
            && command.Execution.Equals("run", StringComparison.OrdinalIgnoreCase))
        {
            var policy = await unitOfWork.BackupPolicies.GetAsync(command.ResourceId, cancellationToken);
            var webhook = policy?.Webhook;
            if (policy is null || webhook is null || !webhook.Enabled)
                return WebhookTarget.NotFound();

            if (webhook.Provider != provider)
                return WebhookTarget.BadRequest("Webhook auth type does not match backup policy webhook provider.");

            return new WebhookTarget(
                Provider: webhook.Provider,
                AuthScheme: webhook.AuthScheme,
                Execution: WebhookExecution.BackupPolicyRun,
                Secret: webhook.Secret,
                BranchFilter: webhook.BranchFilter,
                Repository: null,
                Stack: null,
                GitStack: null,
                Action: null,
                BackupPolicy: policy,
                BuildProject: null,
                Error: null);
        }

        if ((command.ResourceType.Equals("build", StringComparison.OrdinalIgnoreCase)
                || command.ResourceType.Equals("build-project", StringComparison.OrdinalIgnoreCase)
                || command.ResourceType.Equals("buildProject", StringComparison.OrdinalIgnoreCase))
            && command.Execution.Equals("run", StringComparison.OrdinalIgnoreCase))
        {
            var project = await unitOfWork.BuildProjects.GetAsync(command.ResourceId, cancellationToken);
            var webhook = project?.Webhook;
            if (project is null || webhook is null || !webhook.Enabled)
                return WebhookTarget.NotFound();

            if (webhook.Provider != provider)
                return WebhookTarget.BadRequest("Webhook auth type does not match build webhook provider.");

            return new WebhookTarget(
                Provider: webhook.Provider,
                AuthScheme: webhook.AuthScheme,
                Execution: WebhookExecution.BuildRun,
                Secret: webhook.Secret,
                BranchFilter: webhook.BranchFilter ?? project.Branch,
                Repository: null,
                Stack: null,
                GitStack: null,
                Action: null,
                BackupPolicy: null,
                BuildProject: project,
                Error: null);
        }

        return WebhookTarget.BadRequest("Unsupported webhook resource or execution.");
    }

    private async Task<WebhookDispatchResult> DispatchAsync(
        ReceiveWebhook command,
        WebhookTarget target,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        if (payload.UnsupportedEvent)
            return WebhookDispatchResult.NoOp("Unsupported event type");

        return target.Execution switch
        {
            WebhookExecution.RepoPull => await DispatchRepoPullAsync(target, payload, cancellationToken),
            WebhookExecution.StackDeploy => await DispatchStackDeployAsync(target, payload, cancellationToken),
            WebhookExecution.AutomationActionRun => await DispatchAutomationActionRunAsync(command, target, payload, cancellationToken),
            WebhookExecution.BackupPolicyRun => await DispatchBackupPolicyRunAsync(command, target, payload, cancellationToken),
            WebhookExecution.BuildRun => await DispatchBuildRunAsync(target, payload, cancellationToken),
            _ => WebhookDispatchResult.NoOp("Unsupported execution")
        };
    }

    private async Task<WebhookDispatchResult> DispatchRepoPullAsync(
        WebhookTarget target,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        var repo = target.Repository;
        if (repo is null)
            return WebhookDispatchResult.NoOp("Repository not found");

        if (!RepositoryMatches(repo, payload))
            return WebhookDispatchResult.NoOp("Repository identity mismatch");

        var branch = ResolveBranch(target.BranchFilter, payload.Branch, repo.DefaultBranch);
        if (branch.NoOpReason is not null)
            return WebhookDispatchResult.NoOp(branch.NoOpReason);

        repo.MarkProcessing(Constants.SystemId);
        await unitOfWork.GitRepositories.UpdateAsync(repo, cancellationToken);
        await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(gitRepositoryStreamManager, repo), cancellationToken);

        return WebhookDispatchResult.Queued(
            gitSyncRequest: new GitRepoSyncRequest(repo.Id, branch.Branch, GitRepoSyncTrigger.Webhook),
            dispatchedBranch: branch.Branch,
            dispatchedCommitSha: payload.CommitSha);
    }

    private async Task<WebhookDispatchResult> DispatchStackDeployAsync(
        WebhookTarget target,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        var stack = target.Stack;
        var gitStack = target.GitStack;
        if (stack is null || gitStack is null)
            return WebhookDispatchResult.NoOp("Only Git-backed stacks can be deployed by webhook");

        if (!string.IsNullOrWhiteSpace(gitStack.CommitSha))
            return WebhookDispatchResult.NoOp("Stack is pinned to a commit");

        var repo = await unitOfWork.GitRepositories.GetAsync(gitStack.GitRepoId, cancellationToken);
        if (repo is null)
            return WebhookDispatchResult.NoOp("Linked Git repository not found");

        if (!RepositoryMatches(repo, payload))
            return WebhookDispatchResult.NoOp("Repository identity mismatch");

        var branch = ResolveBranch(target.BranchFilter, payload.Branch, gitStack.Branch);
        if (branch.NoOpReason is not null)
            return WebhookDispatchResult.NoOp(branch.NoOpReason);

        var resolvedBranch = branch.Branch ?? gitStack.Branch;
        if (!string.Equals(resolvedBranch, gitStack.Branch, StringComparison.Ordinal))
            return WebhookDispatchResult.NoOp("Branch mismatch");

        var relevance = await ResolveStackWebhookChangeRelevanceAsync(stack, gitStack, repo, resolvedBranch, payload, cancellationToken);
        if (!relevance.Relevant)
            return WebhookDispatchResult.NoOp(relevance.Reason ?? "No relevant path changes");

        var dispatchedCommit = relevance.ResolvedCommitSha ?? payload.CommitSha;

        if (gitStack.UpdateBehavior == StackUpdateBehavior.Disabled)
            return WebhookDispatchResult.NoOp("Stack Git updates are disabled");

        if (gitStack.UpdateBehavior == StackUpdateBehavior.Notify)
        {
            return WebhookDispatchResult.Queued(
                gitSyncRequest: new GitRepoSyncRequest(repo.Id, resolvedBranch, GitRepoSyncTrigger.Webhook),
                dispatchedBranch: resolvedBranch,
                dispatchedCommitSha: dispatchedCommit,
                reason: "Stack update notification queued");
        }

        var entitlementNoOp = await GetEntitlementNoOpAsync(
            LicenseCapability.AutomatedOperations,
            cancellationToken);
        if (entitlementNoOp is not null)
            return entitlementNoOp;

        var release = stack.CurrentStackRelease!;
        return WebhookDispatchResult.Queued(
            gitSyncRequest: null,
            stackDeployQueueItem: StackWebhookDeployQueueItem.Create(
                stack.Id,
                repo.Id,
                release.Id,
                resolvedBranch,
                StackWebhookDeployFingerprint.Compute(gitStack),
                dispatchedCommit,
                DateTime.UtcNow),
            dispatchedBranch: resolvedBranch,
            dispatchedCommitSha: dispatchedCommit);
    }

    private async Task<WebhookDispatchResult> DispatchAutomationActionRunAsync(
        ReceiveWebhook command,
        WebhookTarget target,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        var action = target.Action;
        if (action is null)
            return WebhookDispatchResult.NoOp("Automation action not found");

        var branch = ResolveBranch(target.BranchFilter, payload.Branch, fallbackBranch: null);
        if (branch.NoOpReason is not null)
            return WebhookDispatchResult.NoOp(branch.NoOpReason);

        var entitlementNoOp = await GetEntitlementNoOpAsync(
            LicenseCapability.AutomatedOperations,
            cancellationToken);
        if (entitlementNoOp is not null)
            return entitlementNoOp;

        var result = await automationRunQueueService.QueueAsync(
            action.Id,
            ActionRunTrigger.Webhook,
            NormalizePayload(command.Body),
            timeoutSeconds: null,
            triggeredByActorId: null,
            requireEnabled: true,
            cancellationToken);

        if (result.IsFailure(out var error))
            return WebhookDispatchResult.NoOp(error.Message);

        return WebhookDispatchResult.Queued(
            gitSyncRequest: null,
            dispatchedBranch: branch.Branch,
            dispatchedCommitSha: payload.CommitSha);
    }

    private async Task<WebhookDispatchResult> DispatchBackupPolicyRunAsync(
        ReceiveWebhook command,
        WebhookTarget target,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        var policy = target.BackupPolicy;
        if (policy is null)
            return WebhookDispatchResult.NoOp("Backup policy not found");

        if (!policy.Enabled)
            return WebhookDispatchResult.NoOp("Backup policy is disabled");

        var branch = ResolveBranch(target.BranchFilter, payload.Branch, fallbackBranch: null);
        if (branch.NoOpReason is not null)
            return WebhookDispatchResult.NoOp(branch.NoOpReason);

        var entitlementNoOp = await GetEntitlementNoOpAsync(
            LicenseCapability.AutomatedOperations,
            cancellationToken);
        if (entitlementNoOp is not null)
            return entitlementNoOp;

        var queueResult = await unitOfWork.BackupRuns.QueueAsync(
            policy.Id,
            Guid.CreateVersion7(),
            BackupRunTrigger.Webhook,
            command.ResourceId,
            Constants.SystemId,
            usePolicyActor: true,
            DateTimeOffset.UtcNow,
            cancellationToken);

        if (queueResult.Status != BackupRunQueueResultStatus.Queued)
            return WebhookDispatchResult.NoOp(BackupQueueNoOpReason(queueResult.Status));

        await unitOfWork.CommitAsync(cancellationToken);

        return WebhookDispatchResult.Queued(
            gitSyncRequest: null,
            dispatchedBranch: branch.Branch,
            dispatchedCommitSha: payload.CommitSha);
    }

    private async Task<WebhookDispatchResult> DispatchBuildRunAsync(
        WebhookTarget target,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        var project = target.BuildProject;
        if (project is null)
            return WebhookDispatchResult.NoOp("Build project not found");

        if (!project.Enabled)
            return WebhookDispatchResult.NoOp("Build project is disabled");

        var repo = await unitOfWork.GitRepositories.GetAsync(project.GitRepositoryId, cancellationToken);
        if (repo is null)
            return WebhookDispatchResult.NoOp("Linked Git repository not found");

        if (!RepositoryMatches(repo, payload))
            return WebhookDispatchResult.NoOp("Repository identity mismatch");

        var branch = ResolveBranch(target.BranchFilter, payload.Branch, project.Branch);
        if (branch.NoOpReason is not null)
            return WebhookDispatchResult.NoOp(branch.NoOpReason);

        var automatedNoOp = await GetEntitlementNoOpAsync(
            LicenseCapability.AutomatedOperations,
            cancellationToken);
        if (automatedNoOp is not null)
            return automatedNoOp;

        if (project.BuilderKind == BuildProjectBuilderKind.BuildAgentPool)
        {
            var elasticNoOp = await GetEntitlementNoOpAsync(
                LicenseCapability.ElasticBuildExecution,
                cancellationToken);
            if (elasticNoOp is not null)
                return elasticNoOp;
        }

        var resolvedBranch = branch.Branch ?? project.Branch;
        if (await unitOfWork.BuildRuns.HasActiveRunAsync(project.Id, cancellationToken))
            return WebhookDispatchResult.NoOp("Build project already has an active run");

        var relevance = await ResolveBuildWebhookChangeRelevanceAsync(project, repo, resolvedBranch, payload, cancellationToken);
        if (!relevance.Relevant)
            return WebhookDispatchResult.NoOp(relevance.Reason ?? "No relevant path changes");

        var dispatchedCommit = relevance.ResolvedCommitSha ?? payload.CommitSha;

        var buildTargetResult = await QueueBuildRunHandler.ResolveBuildTargetAsync(
            project,
            unitOfWork,
            cancellationToken);
        if (!buildTargetResult.IsSuccess(out var buildTarget, out var buildTargetError))
            return WebhookDispatchResult.NoOp(buildTargetError!.Message);

        var registry = await unitOfWork.Registries.GetAsync(project.RegistryId, cancellationToken);
        if (registry is null)
            return WebhookDispatchResult.NoOp("Registry not found");

        var imageReferences = QueueBuildRunHandler.ResolveImageReferences(
            registry.RegistryHost,
            project.ImageRepository,
            project.TagTemplates,
            resolvedBranch,
            dispatchedCommit);

        var run = new BuildRun(
            project.Id,
            project.Name,
            repo.Id,
            repo.Name,
            resolvedBranch,
            dispatchedCommit,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.BuildArgs,
            [.. project.BuildSecrets.Select(static s => s.Id)],
            buildTarget.PlatformSnapshot,
            new BuildRegistrySnapshot(registry.Id, registry.Name, registry.RegistryHost),
            project.ImageRepository,
            project.TagTemplates,
            imageReferences,
            BuildRunTrigger.Webhook,
            project.Id,
            Constants.SystemId,
            project.TimeoutSeconds);

        var marked = await unitOfWork.BuildProjects.MarkProcessingAsync(project.Id, run.Id, cancellationToken);
        if (marked == 0)
            return WebhookDispatchResult.NoOp("Build project already has an active run");

        await unitOfWork.BuildRuns.AddAsync(run, cancellationToken);
        var activity = new ActivityEvent(
            platformId: project.PlatformId,
            resourceId: project.Id,
            actorId: Constants.SystemId,
            resourceName: project.Name,
            eventType: ActivityEventType.BuildRunQueued,
            status: ActivityStatus.Success,
            info: new BuildRunQueued(run.Id, run.Trigger));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run, "create");
        var updatedProject = await unitOfWork.BuildProjects.GetAsync(project.Id, cancellationToken);
        if (updatedProject is not null)
            await buildProjectStreamManager.SendBuildProjectInfo(updatedProject, latestRun: run);
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));

        return WebhookDispatchResult.Queued(
            gitSyncRequest: null,
            dispatchedBranch: resolvedBranch,
            dispatchedCommitSha: dispatchedCommit);
    }

    private async Task<(bool Relevant, string? Reason, string? ResolvedCommitSha)> ResolveBuildWebhookChangeRelevanceAsync(
        BuildProject project,
        GitRepository repo,
        string branch,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        if (payload.ChangedPaths.Count > 0)
        {
            return BuildWebhookChangeMatcher.HasRelevantChanges(project, payload.ChangedPaths)
                ? (true, null, payload.CommitSha)
                : (false, "No relevant path changes", null);
        }

        var latestRun = await unitOfWork.BuildRuns.GetLatestByProjectAsync(project.Id, cancellationToken);
        if (latestRun?.Status != BuildRunStatus.Succeeded || string.IsNullOrWhiteSpace(latestRun.ResolvedCommitSha))
            return (true, null, payload.CommitSha);

        var sync = await repoCacheManager.SynchronizeAsync(repo, repo.GitAccount, branch, cancellationToken);
        if (sync.Success != true || string.IsNullOrWhiteSpace(sync.Hash))
            return (false, sync.Error ?? "Repository sync did not resolve a commit", null);

        if (string.Equals(latestRun.ResolvedCommitSha, sync.Hash, StringComparison.OrdinalIgnoreCase))
            return (false, "No new commit", null);

        var pathsResult = await gitCliRepository.GetChangedPathsAsync(
            ApplicationStoragePaths.GetRepositoryCachePath(repo),
            latestRun.ResolvedCommitSha,
            sync.Hash,
            cancellationToken);

        if (pathsResult.IsFailure(out _, out var changedPaths))
            return (true, null, sync.Hash);

        return BuildWebhookChangeMatcher.HasRelevantChanges(project, changedPaths)
            ? (true, null, sync.Hash)
            : (false, "No relevant path changes", null);
    }

    private async Task<(bool Relevant, string? Reason, string? ResolvedCommitSha)> ResolveStackWebhookChangeRelevanceAsync(
        Stack stack,
        GitStack gitStack,
        GitRepository repo,
        string branch,
        WebhookPayloadInfo payload,
        CancellationToken cancellationToken)
    {
        if (gitStack.Webhook?.ForceDeploy == true)
            return (true, null, payload.CommitSha);

        if (stack.CurrentStackRelease?.Source is not { } source)
            return (true, null, payload.CommitSha);

        if (payload.ChangedPaths.Count > 0)
        {
            return GitStackWatchPathMatcher.HasRelevantChanges(gitStack, source, payload.ChangedPaths)
                ? (true, null, payload.CommitSha)
                : (false, "No relevant path changes", null);
        }

        if (string.IsNullOrWhiteSpace(source.ResolvedCommitSha))
            return (true, null, payload.CommitSha);

        var sync = await repoCacheManager.SynchronizeAsync(repo, repo.GitAccount, branch, cancellationToken);
        if (sync.Success != true || string.IsNullOrWhiteSpace(sync.Hash))
            return (false, sync.Error ?? "Repository sync did not resolve a commit", null);

        if (string.Equals(source.ResolvedCommitSha, sync.Hash, StringComparison.OrdinalIgnoreCase))
            return (false, "No new commit", null);

        var pathsResult = await gitCliRepository.GetChangedPathsAsync(
            ApplicationStoragePaths.GetRepositoryCachePath(repo),
            source.ResolvedCommitSha,
            sync.Hash,
            cancellationToken);

        if (pathsResult.IsFailure(out _, out var changedPaths))
            return (true, null, sync.Hash);

        return GitStackWatchPathMatcher.HasRelevantChanges(gitStack, source, changedPaths)
            ? (true, null, sync.Hash)
            : (false, "No relevant path changes", null);
    }

    private async Task RecordActivityAsync(
        ReceiveWebhook command,
        Guid requestId,
        WebhookTarget? target,
        WebhookPayloadInfo? payload,
        WebhookDispatchResult dispatch,
        string status,
        string? reason,
        ActivityStatus activityStatus,
        CancellationToken cancellationToken)
    {
        ActivityEvent? activity = null;
        TryResolveActivityEvent(
                command,
                target,
                requestId,
                payload,
                status,
                reason,
                activityStatus,
                dispatch.DispatchedBranch,
                dispatch.DispatchedCommitSha,
                out activity);

        if (activity is null && dispatch.StackDeployQueueItem is null)
            return;

        if (activity is not null)
            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        if (status.Equals("queued", StringComparison.OrdinalIgnoreCase)
            && dispatch.StackDeployQueueItem is { } queueItem)
        {
            await unitOfWork.StackWebhookDeployQueue.AddAsync(queueItem, cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        if (dispatch.StackDeployQueueItem is { } persistedQueueItem)
            stackDeployWriter.TryWrite(new StackWebhookDeploySignal(persistedQueueItem.Id));

        if (activity is not null)
        {
            await notificationQueue.EnqueueAsync(
                new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(unitOfWork, cancellationToken)),
                cancellationToken);
        }

        if (status.Equals("queued", StringComparison.OrdinalIgnoreCase)
            && dispatch.GitSyncRequest is { } syncRequest)
        {
            await gitSyncWriter.WriteAsync(syncRequest, cancellationToken);
        }
    }

    private Task ProcessWebhookAlertAsync(
        AlertType alertType,
        ReceiveWebhook command,
        Guid requestId,
        WebhookTarget target,
        WebhookPayloadInfo? payload,
        string reason,
        CancellationToken cancellationToken)
    {
        var snapshot = CreateWebhookAlertSnapshot(command, requestId, target, payload, reason);
        if (snapshot is null)
            return Task.CompletedTask;

        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            Webhooks: [snapshot]);

        return alertService.ProcessAsync(alertType, context, cancellationToken);
    }

    private static WebhookAlertSnapshot? CreateWebhookAlertSnapshot(
        ReceiveWebhook command,
        Guid requestId,
        WebhookTarget target,
        WebhookPayloadInfo? payload,
        string reason)
    {
        var provider = target.Provider.ToString();
        if (target.Repository is { } repo)
        {
            return new WebhookAlertSnapshot(
                repo.Id,
                repo.Name,
                AlertResourceType.Webhook,
                "repository",
                provider,
                command.Execution,
                reason,
                requestId,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName);
        }

        if (target.Stack is { } stack)
        {
            return new WebhookAlertSnapshot(
                stack.Id,
                stack.Name,
                AlertResourceType.Webhook,
                "stack",
                provider,
                command.Execution,
                reason,
                requestId,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName);
        }

        if (target.Action is { } action)
        {
            return new WebhookAlertSnapshot(
                action.Id,
                action.Name,
                AlertResourceType.Webhook,
                "automation-action",
                provider,
                command.Execution,
                reason,
                requestId,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName);
        }

        if (target.BackupPolicy is { } backupPolicy)
        {
            return new WebhookAlertSnapshot(
                backupPolicy.Id,
                backupPolicy.Name,
                AlertResourceType.Webhook,
                "backup-policy",
                provider,
                command.Execution,
                reason,
                requestId,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName);
        }

        if (target.BuildProject is { } buildProject)
        {
            return new WebhookAlertSnapshot(
                buildProject.Id,
                buildProject.Name,
                AlertResourceType.Webhook,
                "build",
                provider,
                command.Execution,
                reason,
                requestId,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName);
        }

        return null;
    }

    private static bool IsAlertableDispatchNoOp(string? reason)
        => reason is not null
           && !reason.StartsWith("Paused by license:", StringComparison.OrdinalIgnoreCase)
           && !reason.Equals("Branch mismatch", StringComparison.OrdinalIgnoreCase)
           && !reason.Equals("No relevant path changes", StringComparison.OrdinalIgnoreCase)
           && !reason.Equals("No new commit", StringComparison.OrdinalIgnoreCase)
           && !reason.Equals("Unsupported event type", StringComparison.OrdinalIgnoreCase);

    private async Task<WebhookDispatchResult?> GetEntitlementNoOpAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken)
    {
        var entitlement = await entitlementService.EnsureEnabledAsync(capability, cancellationToken);
        return entitlement.IsFailure(out var error)
            ? WebhookDispatchResult.NoOp($"Paused by license: {error.Message}")
            : null;
    }

    private static string BackupQueueNoOpReason(BackupRunQueueResultStatus status)
        => status switch
        {
            BackupRunQueueResultStatus.PolicyNotFound => "Backup policy not found",
            BackupRunQueueResultStatus.PolicyArchived => "Archived backup policies cannot queue new runs",
            BackupRunQueueResultStatus.ActiveRunExists => "Backup policy already has an active run",
            _ => "Backup run could not be queued"
        };

    private static bool TryResolveActivityEvent(
        ReceiveWebhook command,
        WebhookTarget? target,
        Guid requestId,
        WebhookPayloadInfo? payload,
        string status,
        string? reason,
        ActivityStatus activityStatus,
        string? dispatchedBranch,
        string? dispatchedCommitSha,
        out ActivityEvent activity)
    {
        activity = null!;
        if (command.ResourceType.Equals("repo", StringComparison.OrdinalIgnoreCase))
        {
            var info = new GitRepoWebhookReceived(
                requestId,
                command.AuthType,
                command.Execution,
                status,
                reason,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName,
                dispatchedBranch,
                dispatchedCommitSha);

            activity = new ActivityEvent(
                platformId: null,
                resourceId: target?.Repository?.Id ?? command.ResourceId,
                actorId: Constants.SystemId,
                resourceName: target?.Repository?.Name ?? $"repo:{command.ResourceId}",
                eventType: ActivityEventType.GitRepoWebhookReceived,
                status: activityStatus,
                info: info);
            return true;
        }

        if (command.ResourceType.Equals("stack", StringComparison.OrdinalIgnoreCase))
        {
            var info = new StackWebhookReceived(
                requestId,
                command.AuthType,
                command.Execution,
                status,
                reason,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName,
                dispatchedBranch,
                dispatchedCommitSha);

            activity = new ActivityEvent(
                platformId: target?.Stack?.CurrentStackRelease?.PlatformId,
                resourceId: target?.Stack?.Id ?? command.ResourceId,
                actorId: Constants.SystemId,
                resourceName: target?.Stack?.Name ?? $"stack:{command.ResourceId}",
                eventType: ActivityEventType.StackWebhookReceived,
                status: activityStatus,
                info: info);
            return true;
        }

        if (command.ResourceType.Equals("build", StringComparison.OrdinalIgnoreCase)
            || command.ResourceType.Equals("build-project", StringComparison.OrdinalIgnoreCase)
            || command.ResourceType.Equals("buildProject", StringComparison.OrdinalIgnoreCase))
        {
            var info = new BuildWebhookReceived(
                requestId,
                command.AuthType,
                command.Execution,
                status,
                reason,
                payload?.EventType,
                payload?.DeliveryId,
                payload?.Branch,
                payload?.CommitSha,
                payload?.RepositoryFullName,
                dispatchedBranch,
                dispatchedCommitSha);

            activity = new ActivityEvent(
                platformId: target?.BuildProject?.PlatformId,
                resourceId: target?.BuildProject?.Id ?? command.ResourceId,
                actorId: Constants.SystemId,
                resourceName: target?.BuildProject?.Name ?? $"build:{command.ResourceId}",
                eventType: ActivityEventType.BuildWebhookReceived,
                status: activityStatus,
                info: info);
            return true;
        }

        return false;
    }

    private static bool TryResolveProvider(string authType, out WebhookProvider provider)
    {
        if (authType.Equals("github", StringComparison.OrdinalIgnoreCase))
        {
            provider = WebhookProvider.GitHub;
            return true;
        }

        if (authType.Equals("gitlab", StringComparison.OrdinalIgnoreCase))
        {
            provider = WebhookProvider.GitLab;
            return true;
        }

        provider = default;
        return false;
    }

    private static (string? Branch, string? NoOpReason) ResolveBranch(string? filter, string? payloadBranch, string? fallbackBranch)
    {
        if (!string.IsNullOrWhiteSpace(filter))
        {
            if (string.IsNullOrWhiteSpace(payloadBranch))
                return (null, "Payload branch missing");

            return string.Equals(filter, payloadBranch, StringComparison.Ordinal)
                ? (payloadBranch, null)
                : (null, "Branch mismatch");
        }

        return (!string.IsNullOrWhiteSpace(payloadBranch) ? payloadBranch : fallbackBranch, null);
    }

    private static bool RepositoryMatches(GitRepository repository, WebhookPayloadInfo payload)
    {
        if (payload.RepositoryUrls.Count == 0 && string.IsNullOrWhiteSpace(payload.RepositoryFullName))
            return true;

        var normalizedRepoUrl = NormalizeRepositoryIdentity(repository.Url);
        if (payload.RepositoryUrls.Select(NormalizeRepositoryIdentity).Any(url => string.Equals(url, normalizedRepoUrl, StringComparison.OrdinalIgnoreCase)))
            return true;

        if (!string.IsNullOrWhiteSpace(payload.RepositoryFullName))
        {
            var normalizedRepoName = NormalizeRepositoryIdentity(payload.RepositoryFullName);
            return normalizedRepoUrl.EndsWith(normalizedRepoName, StringComparison.OrdinalIgnoreCase);
        }

        return false;
    }

    private static string NormalizeRepositoryIdentity(string? value)
        => (value ?? string.Empty)
            .Trim()
            .TrimEnd('/')
            .Replace(".git", string.Empty, StringComparison.OrdinalIgnoreCase)
            .ToLowerInvariant();

    private static string NormalizePayload(byte[] rawBody)
    {
        if (rawBody.Length == 0)
            return "{}";

        var payload = Encoding.UTF8.GetString(rawBody);
        if (string.IsNullOrWhiteSpace(payload))
            return "{}";

        using var document = JsonDocument.Parse(rawBody);
        return document.RootElement.ValueKind == JsonValueKind.Object
            ? payload
            : $$"""{"payload":{{payload}}}""";
    }

    private static bool Authenticate(WebhookAuthScheme authScheme, IReadOnlyDictionary<string, string[]> headers, byte[] rawBody, string? secret)
    {
        if (string.IsNullOrWhiteSpace(secret))
            return true;

        return authScheme switch
        {
            WebhookAuthScheme.GitHubHmacSha256 => ValidateGitHub(headers, rawBody, secret),
            WebhookAuthScheme.GitLabSignedToken => ValidateGitLabSigned(headers, rawBody, secret),
            WebhookAuthScheme.GitLabLegacyToken => ValidateGitLabLegacy(headers, secret),
            _ => false
        };
    }

    private static bool ValidateGitHub(IReadOnlyDictionary<string, string[]> headers, byte[] rawBody, string secret)
    {
        var header = GetHeader(headers, "X-Hub-Signature-256");
        var expectedBytes = HMACSHA256.HashData(Encoding.UTF8.GetBytes(secret), rawBody);
        var expectedHex = Convert.ToHexString(expectedBytes).ToLowerInvariant();

        if (string.IsNullOrWhiteSpace(header))
        {
            header = GetHeader(headers, "X-Gitea-Signature") ?? GetHeader(headers, "X-Forgejo-Signature");
            if (string.IsNullOrWhiteSpace(header))
                return false;

            return FixedTimeEquals(header.Trim(), expectedHex);
        }

        if (!header.StartsWith("sha256=", StringComparison.OrdinalIgnoreCase))
            return false;

        return FixedTimeEquals(header.Trim(), $"sha256={expectedHex}");
    }

    private static bool FixedTimeEquals(string actualValue, string expectedValue)
    {
        var expected = Encoding.ASCII.GetBytes(expectedValue);
        var actual = Encoding.ASCII.GetBytes(actualValue);
        return actual.Length == expected.Length && CryptographicOperations.FixedTimeEquals(actual, expected);
    }

    private static bool ValidateGitLabLegacy(IReadOnlyDictionary<string, string[]> headers, string secret)
    {
        var token = GetHeader(headers, "X-Gitlab-Token");
        if (string.IsNullOrWhiteSpace(token))
            return false;

        var expected = Encoding.UTF8.GetBytes(secret);
        var actual = Encoding.UTF8.GetBytes(token);
        return actual.Length == expected.Length && CryptographicOperations.FixedTimeEquals(actual, expected);
    }

    private static bool ValidateGitLabSigned(IReadOnlyDictionary<string, string[]> headers, byte[] rawBody, string secret)
    {
        var webhookId = GetHeader(headers, "webhook-id");
        var timestamp = GetHeader(headers, "webhook-timestamp");
        var signatureHeader = GetHeader(headers, "webhook-signature");
        if (string.IsNullOrWhiteSpace(webhookId) || string.IsNullOrWhiteSpace(timestamp) || string.IsNullOrWhiteSpace(signatureHeader))
            return false;

        if (!long.TryParse(timestamp, out var unixTimestamp))
            return false;

        var signedAt = DateTimeOffset.FromUnixTimeSeconds(unixTimestamp);
        if (DateTimeOffset.UtcNow - signedAt > GitLabSignedTimestampTolerance || signedAt - DateTimeOffset.UtcNow > GitLabSignedTimestampTolerance)
            return false;

        if (!secret.StartsWith("whsec_", StringComparison.Ordinal))
            return false;

        byte[] key;
        try
        {
            key = Convert.FromBase64String(secret["whsec_".Length..]);
        }
        catch
        {
            return false;
        }

        var prefix = Encoding.UTF8.GetBytes($"{webhookId}.{timestamp}.");
        var signedPayload = new byte[prefix.Length + rawBody.Length];
        Buffer.BlockCopy(prefix, 0, signedPayload, 0, prefix.Length);
        Buffer.BlockCopy(rawBody, 0, signedPayload, prefix.Length, rawBody.Length);

        var expected = Encoding.ASCII.GetBytes($"v1,{Convert.ToBase64String(HMACSHA256.HashData(key, signedPayload))}");
        foreach (var candidate in signatureHeader.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            var actual = Encoding.ASCII.GetBytes(candidate);
            if (actual.Length == expected.Length && CryptographicOperations.FixedTimeEquals(actual, expected))
                return true;
        }

        return false;
    }

    private static WebhookPayloadInfo ParsePayload(WebhookProvider provider, IReadOnlyDictionary<string, string[]> headers, byte[] rawBody)
    {
        var deliveryId = provider == WebhookProvider.GitHub
            ? GetHeader(headers, "X-GitHub-Delivery")
            : GetHeader(headers, "webhook-id")
              ?? GetHeader(headers, "Idempotency-Key")
              ?? GetHeader(headers, "X-Gitlab-Webhook-UUID")
              ?? GetHeader(headers, "X-Gitlab-Event-UUID");

        var eventType = provider == WebhookProvider.GitHub
            ? GetHeader(headers, "X-GitHub-Event")
            : GetHeader(headers, "X-Gitlab-Event");

        if (!string.IsNullOrWhiteSpace(eventType)
            && !eventType.Contains("push", StringComparison.OrdinalIgnoreCase)
            && !eventType.Equals("Push Hook", StringComparison.OrdinalIgnoreCase))
        {
            return new WebhookPayloadInfo(deliveryId, eventType, null, null, null, [], [], true);
        }

        try
        {
            using var document = JsonDocument.Parse(rawBody);
            var root = document.RootElement;
            var branch = TryGetString(root, "ref") is { } reference && reference.StartsWith("refs/heads/", StringComparison.Ordinal)
                ? reference["refs/heads/".Length..]
                : null;
            var commit = TryGetString(root, provider == WebhookProvider.GitLab ? "checkout_sha" : "after")
                         ?? TryGetString(root, "after");
            var repo = provider == WebhookProvider.GitHub && root.TryGetProperty("repository", out var ghRepo)
                ? ghRepo
                : provider == WebhookProvider.GitLab && root.TryGetProperty("project", out var glProject)
                    ? glProject
                    : default;

            var urls = new List<string>();
            string? fullName = null;
            if (repo.ValueKind == JsonValueKind.Object)
            {
                AddIfNotEmpty(urls, TryGetString(repo, "html_url"));
                AddIfNotEmpty(urls, TryGetString(repo, "clone_url"));
                AddIfNotEmpty(urls, TryGetString(repo, "ssh_url"));
                AddIfNotEmpty(urls, TryGetString(repo, "git_http_url"));
                AddIfNotEmpty(urls, TryGetString(repo, "git_ssh_url"));
                fullName = TryGetString(repo, "full_name") ?? TryGetString(repo, "path_with_namespace");
            }

            var changedPaths = GetChangedPaths(root);
            return new WebhookPayloadInfo(deliveryId, eventType, branch, commit, fullName, urls, changedPaths, false);
        }
        catch
        {
            return new WebhookPayloadInfo(deliveryId, eventType, null, null, null, [], [], true);
        }
    }

    private static IReadOnlyList<string> GetChangedPaths(JsonElement root)
    {
        if (!root.TryGetProperty("commits", out var commits) || commits.ValueKind != JsonValueKind.Array)
            return [];

        var changedPaths = new HashSet<string>(StringComparer.Ordinal);
        foreach (var commit in commits.EnumerateArray())
        {
            AddPathArray(changedPaths, commit, "added");
            AddPathArray(changedPaths, commit, "modified");
            AddPathArray(changedPaths, commit, "removed");
        }

        return changedPaths.ToArray();
    }

    private static void AddPathArray(HashSet<string> paths, JsonElement element, string property)
    {
        if (!element.TryGetProperty(property, out var values) || values.ValueKind != JsonValueKind.Array)
            return;

        foreach (var value in values.EnumerateArray())
        {
            if (value.ValueKind != JsonValueKind.String)
                continue;

            var path = value.GetString()?.Replace('\\', '/').Trim().TrimStart('/');
            if (!string.IsNullOrWhiteSpace(path))
                paths.Add(path);
        }
    }

    private static string? GetHeader(IReadOnlyDictionary<string, string[]> headers, string name)
        => headers.TryGetValue(name, out var values) ? values.FirstOrDefault() : null;

    private static string? TryGetString(JsonElement element, string property)
        => element.ValueKind == JsonValueKind.Object
           && element.TryGetProperty(property, out var value)
           && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    private static void AddIfNotEmpty(List<string> values, string? value)
    {
        if (!string.IsNullOrWhiteSpace(value))
            values.Add(value);
    }

    private sealed record WebhookTarget(
        WebhookProvider Provider,
        WebhookAuthScheme AuthScheme,
        WebhookExecution Execution,
        string? Secret,
        string? BranchFilter,
        GitRepository? Repository,
        Stack? Stack,
        GitStack? GitStack,
        AutomationAction? Action,
        BackupPolicy? BackupPolicy,
        BuildProject? BuildProject,
        Error? Error)
    {
        public static WebhookTarget NotFound()
            => new(default, default, default, null, null, null, null, null, null, null, null, new NotFoundError("Webhook target not found."));

        public static WebhookTarget BadRequest(string reason)
            => new(default, default, default, null, null, null, null, null, null, null, null, new BadRequestError(reason));
    }

    private sealed record WebhookPayloadInfo(
        string? DeliveryId,
        string? EventType,
        string? Branch,
        string? CommitSha,
        string? RepositoryFullName,
        IReadOnlyList<string> RepositoryUrls,
        IReadOnlyList<string> ChangedPaths,
        bool UnsupportedEvent);

    private sealed record WebhookDispatchResult(
        string Status,
        string? Reason,
        GitRepoSyncRequest? GitSyncRequest = null,
        StackWebhookDeployQueueItem? StackDeployQueueItem = null,
        string? DispatchedBranch = null,
        string? DispatchedCommitSha = null)
    {
        public static WebhookDispatchResult Queued(
            GitRepoSyncRequest? gitSyncRequest = null,
            StackWebhookDeployQueueItem? stackDeployQueueItem = null,
            string? dispatchedBranch = null,
            string? dispatchedCommitSha = null,
            string? reason = null)
            => new("queued", reason, gitSyncRequest, stackDeployQueueItem, dispatchedBranch, dispatchedCommitSha);

        public static WebhookDispatchResult NoOp(string reason) => new("noop", reason);
    }
}

internal static class BuildWebhookChangeMatcher
{
    internal static bool HasRelevantChanges(BuildProject project, IReadOnlyList<string> changedPaths)
    {
        var contextPath = NormalizePath(project.ContextPath);
        var dockerfilePath = NormalizePath(project.DockerfilePath);

        return changedPaths.Any(path =>
        {
            var normalized = NormalizePath(path);
            return IsSameOrChild(normalized, contextPath)
                   || string.Equals(normalized, dockerfilePath, StringComparison.Ordinal);
        });
    }

    private static string NormalizePath(string? value)
        => string.IsNullOrWhiteSpace(value)
            ? "."
            : value.Replace('\\', '/').Trim().TrimStart('/').TrimEnd('/');

    private static bool IsSameOrChild(string path, string directory)
        => directory is "." or ""
           || string.Equals(path, directory, StringComparison.Ordinal)
           || path.StartsWith(directory + "/", StringComparison.Ordinal);
}
