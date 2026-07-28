using Application.Services.Alerts;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Application.Services;

internal sealed class StackWebhookDeployProcessor(
    StackWebhookDeployQueueService queue,
    IServiceScopeFactory scopeFactory,
    IApplyStackService applyStackService,
    IAlertService alertService,
    ILicenseEntitlementService entitlementService,
    TimeProvider timeProvider,
    ILogger<StackWebhookDeployProcessor> logger)
{
    private const int MaxAttempts = 3;

    internal async Task ProcessAsync(Guid id, CancellationToken cancellationToken)
    {
        var item = await queue.TryClaimAsync(id, cancellationToken);
        if (item is null)
            return;

        try
        {
            var validation = await ValidateAsync(item, cancellationToken);
            if (validation != StackWebhookDeployValidation.Ready)
            {
                logger.LogInformation(
                    "Skipping webhook stack deploy job {JobId} for stack {StackId}: {Reason}.",
                    item.Id,
                    item.StackId,
                    validation);
                await queue.DeleteAsync(item.Id, cancellationToken);
                return;
            }

            if (!await entitlementService.IsEnabledAsync(
                    LicenseCapability.AutomatedOperations,
                    cancellationToken))
            {
                logger.LogInformation(
                    "Skipping webhook stack deploy job {JobId} because automated operations are no longer licensed.",
                    item.Id);
                await queue.DeleteAsync(item.Id, cancellationToken);
                return;
            }

            var failure = await ApplyAsync(item, cancellationToken);
            if (failure is null)
            {
                await queue.DeleteAsync(item.Id, cancellationToken);
                return;
            }

            await HandleFailureAsync(item, failure, cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            await queue.RetryAsync(
                item.Id,
                "Interrupted by application shutdown.",
                timeProvider.GetUtcNow().UtcDateTime,
                CancellationToken.None);
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Webhook stack deploy job {JobId} failed for stack {StackId}.",
                item.Id,
                item.StackId);
            await HandleFailureAsync(item, ex.Message, CancellationToken.None);
        }
    }

    private async Task<string?> ApplyAsync(
        StackWebhookDeployQueueItem item,
        CancellationToken cancellationToken)
    {
        await foreach (var result in applyStackService.ApplyAsync(
                           item.StackId,
                           Constants.SystemId,
                           serviceNames: null,
                           pullImages: true,
                           recreate: false,
                           waitForCompletion: true,
                           operation: StackApplyOperation.Apply,
                           previousStackSnapshot: null,
                           cancellationToken))
        {
            var failure = GetDeployFailure(result);
            if (failure is not null)
                return failure;
        }

        return null;
    }

    private async Task HandleFailureAsync(
        StackWebhookDeployQueueItem item,
        string reason,
        CancellationToken cancellationToken)
    {
        if (item.Attempts < MaxAttempts)
        {
            var delay = TimeSpan.FromSeconds(5 * (1 << Math.Max(0, item.Attempts - 1)));
            await queue.RetryAsync(
                item.Id,
                reason,
                timeProvider.GetUtcNow().UtcDateTime.Add(delay),
                cancellationToken);
            return;
        }

        await TryRaiseFailureAlertAsync(item, reason, cancellationToken);
        await queue.DeleteAsync(item.Id, cancellationToken);
    }

    private async Task<StackWebhookDeployValidation> ValidateAsync(
        StackWebhookDeployQueueItem item,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(item.StackId, cancellationToken);
        if (stack?.CurrentStackRelease is not { } release
            || release.Id != item.ExpectedStackReleaseId
            || release.Spec is not GitStack gitStack)
        {
            return StackWebhookDeployValidation.StackChanged;
        }

        if (gitStack.GitRepoId != item.GitRepositoryId
            || !string.Equals(gitStack.Branch, item.Branch, StringComparison.Ordinal)
            || !string.IsNullOrWhiteSpace(gitStack.CommitSha)
            || gitStack.UpdateBehavior != StackUpdateBehavior.StackAutoDeploy
            || gitStack.Webhook?.Enabled != true
            || !string.Equals(
                StackWebhookDeployFingerprint.Compute(gitStack),
                item.ExpectedSpecFingerprint,
                StringComparison.Ordinal))
        {
            return StackWebhookDeployValidation.StackChanged;
        }

        if (!string.IsNullOrWhiteSpace(item.DispatchedCommitSha)
            && string.Equals(
                release.Source?.ResolvedCommitSha,
                item.DispatchedCommitSha,
                StringComparison.OrdinalIgnoreCase))
        {
            return StackWebhookDeployValidation.AlreadyApplied;
        }

        return StackWebhookDeployValidation.Ready;
    }

    private async Task TryRaiseFailureAlertAsync(
        StackWebhookDeployQueueItem item,
        string reason,
        CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stack = await uow.Stacks.GetAsync(item.StackId, cancellationToken);
            var repository = await uow.GitRepositories.GetAsync(
                item.GitRepositoryId,
                cancellationToken);
            if (stack is null || repository is null)
                return;

            var context = new AlertEvaluationContext(
                UtcNow: timeProvider.GetUtcNow().UtcDateTime,
                Platforms: [],
                Deployments: [],
                Stacks: [],
                StackGitWebhookDeployFailures:
                [
                    new StackGitWebhookDeployFailureAlertSnapshot(
                        stack.Id,
                        stack.Name,
                        repository.Name,
                        item.Branch,
                        reason)
                ]);

            await alertService.ProcessAsync(
                AlertType.WebhookStackGitDeployFailed,
                context,
                cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Failed to raise the webhook deploy failure alert for stack {StackId}.",
                item.StackId);
        }
    }

    private static string? GetDeployFailure(StackStreamItem item)
    {
        if (item.Type != StackApplyEventType.StdErr
            && item.StackStatus != StackReleaseStatus.Failed
            && item.ExitCode is not > 0)
        {
            return null;
        }

        return !string.IsNullOrWhiteSpace(item.Message)
            ? item.Message
            : !string.IsNullOrWhiteSpace(item.ProgressMessage)
                ? item.ProgressMessage
                : item.ExitCode is int exitCode
                    ? $"Stack apply exited with code {exitCode}."
                    : "Stack apply failed.";
    }

    private enum StackWebhookDeployValidation
    {
        Ready,
        StackChanged,
        AlreadyApplied
    }
}
