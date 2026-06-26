using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;
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
    INotificationQueue notificationQueue,
    IGitRepositoryStreamManager gitRepositoryStreamManager,
    IActivityStreamManager activityStreamManager,
    IAlertService alertService,
    IApplyStackService applyStackService,
    ILoggerFactory loggerFactory) : ICommandHandler<ReceiveWebhook, Result<WebhookReceiveResult>>
{
    private const int MaxBodyBytes = 1024 * 1024;
    private static readonly TimeSpan GitLabSignedTimestampTolerance = TimeSpan.FromMinutes(5);

    public async ValueTask<Result<WebhookReceiveResult>> Handle(ReceiveWebhook command, CancellationToken cancellationToken)
    {
        var requestId = Guid.CreateVersion7();
        if (command.Body.Length > MaxBodyBytes)
        {
            await RecordActivityAsync(command, requestId, target: null, payload: null, pendingGitSyncRequest: null, status: "rejected", reason: "Request body too large", ActivityStatus.Failure, cancellationToken);
            return Result.Failure<WebhookReceiveResult>(new BadRequestError("Request body too large"));
        }

        var target = await ResolveTargetAsync(command, cancellationToken);
        if (target.Error is not null)
        {
            await RecordActivityAsync(command, requestId, target, payload: null, pendingGitSyncRequest: null, status: "rejected", reason: target.Error.Message, ActivityStatus.Failure, cancellationToken);
            return Result.Failure<WebhookReceiveResult>(target.Error);
        }

        if (!Authenticate(target.AuthScheme, command.Headers, command.Body, target.Secret))
        {
            await RecordActivityAsync(command, requestId, target, payload: null, pendingGitSyncRequest: null, status: "rejected", reason: "Webhook authentication failed", ActivityStatus.Failure, cancellationToken);
            await ProcessWebhookAlertAsync(AlertType.WebhookAuthenticationFailed, command, requestId, target, payload: null, "Webhook authentication failed", cancellationToken);
            return Result.Failure<WebhookReceiveResult>(new UnauthorizedError("Webhook authentication failed"));
        }

        var payload = ParsePayload(target.Provider, command.Headers, command.Body);
        var dispatch = await DispatchAsync(target, payload, cancellationToken);
        await RecordActivityAsync(
            command,
            requestId,
            target,
            payload,
            dispatch.GitSyncRequest,
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
                Error: null);
        }

        return WebhookTarget.BadRequest("Unsupported webhook resource or execution.");
    }

    private async Task<WebhookDispatchResult> DispatchAsync(
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

        return WebhookDispatchResult.Queued(new GitRepoSyncRequest(repo.Id, branch.Branch, GitRepoSyncTrigger.Webhook));
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
        var logger = loggerFactory.CreateLogger("WebhookStackDeploy");
        _ = Task.Run(async () =>
        {
            try
            {
                await foreach (var item in applyStackService.ApplyAsync(
                    stack.Id,
                    Constants.SystemId,
                    serviceNames: null,
                    pullImages: true,
                    StackApplyOperation.Apply,
                    previousStackSnapshot: null,
                    CancellationToken.None))
                {
                    if (!string.IsNullOrWhiteSpace(item.Message))
                    {
                        await ProcessStackWebhookDeployFailureAlertAsync(stack, repo, resolvedBranch, item.Message, CancellationToken.None);
                        return;
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Webhook stack deploy failed for stack {StackId}", stack.Id);
                await ProcessStackWebhookDeployFailureAlertAsync(stack, repo, resolvedBranch, ex.Message, CancellationToken.None);
            }
        }, CancellationToken.None);

        return WebhookDispatchResult.Queued();
    }

    private async Task RecordActivityAsync(
        ReceiveWebhook command,
        Guid requestId,
        WebhookTarget? target,
        WebhookPayloadInfo? payload,
        GitRepoSyncRequest? pendingGitSyncRequest,
        string status,
        string? reason,
        ActivityStatus activityStatus,
        CancellationToken cancellationToken)
    {
        if (!TryResolveActivityEvent(command, target, requestId, payload, status, reason, activityStatus, out var activity))
            return;

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await notificationQueue.EnqueueAsync(
            new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(unitOfWork, cancellationToken)),
            cancellationToken);

        if (status.Equals("queued", StringComparison.OrdinalIgnoreCase)
            && target?.Execution == WebhookExecution.RepoPull
            && pendingGitSyncRequest is { } syncRequest)
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

    private Task ProcessStackWebhookDeployFailureAlertAsync(
        Stack stack,
        GitRepository repository,
        string branch,
        string reason,
        CancellationToken cancellationToken)
    {
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            StackGitWebhookDeployFailures:
            [
                new StackGitWebhookDeployFailureAlertSnapshot(
                    stack.Id,
                    stack.Name,
                    repository.Name,
                    branch,
                    reason)
            ]);

        return alertService.ProcessAsync(AlertType.WebhookStackGitDeployFailed, context, cancellationToken);
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

        return null;
    }

    private static bool IsAlertableDispatchNoOp(string? reason)
        => reason is not null
           && !reason.Equals("Branch mismatch", StringComparison.OrdinalIgnoreCase)
           && !reason.Equals("Unsupported event type", StringComparison.OrdinalIgnoreCase);

    private static bool TryResolveActivityEvent(
        ReceiveWebhook command,
        WebhookTarget? target,
        Guid requestId,
        WebhookPayloadInfo? payload,
        string status,
        string? reason,
        ActivityStatus activityStatus,
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
                payload?.RepositoryFullName);

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
                payload?.RepositoryFullName);

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
        if (string.IsNullOrWhiteSpace(header) || !header.StartsWith("sha256=", StringComparison.OrdinalIgnoreCase))
            return false;

        var expectedBytes = HMACSHA256.HashData(Encoding.UTF8.GetBytes(secret), rawBody);
        var expected = Encoding.ASCII.GetBytes($"sha256={Convert.ToHexString(expectedBytes).ToLowerInvariant()}");
        var actual = Encoding.ASCII.GetBytes(header.Trim());
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
            return new WebhookPayloadInfo(deliveryId, eventType, null, null, null, [], true);
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

            return new WebhookPayloadInfo(deliveryId, eventType, branch, commit, fullName, urls, false);
        }
        catch
        {
            return new WebhookPayloadInfo(deliveryId, eventType, null, null, null, [], true);
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
        Error? Error)
    {
        public static WebhookTarget NotFound()
            => new(default, default, default, null, null, null, null, null, new NotFoundError("Webhook target not found."));

        public static WebhookTarget BadRequest(string reason)
            => new(default, default, default, null, null, null, null, null, new BadRequestError(reason));
    }

    private sealed record WebhookPayloadInfo(
        string? DeliveryId,
        string? EventType,
        string? Branch,
        string? CommitSha,
        string? RepositoryFullName,
        IReadOnlyList<string> RepositoryUrls,
        bool UnsupportedEvent);

    private sealed record WebhookDispatchResult(string Status, string? Reason, GitRepoSyncRequest? GitSyncRequest = null)
    {
        public static WebhookDispatchResult Queued(GitRepoSyncRequest? gitSyncRequest = null) => new("queued", null, gitSyncRequest);
        public static WebhookDispatchResult NoOp(string reason) => new("noop", reason);
    }
}
