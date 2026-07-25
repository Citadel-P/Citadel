using Application.Services.Licensing;
using Domain;
using Domain.Entities.Stacks;
using LightResults;

namespace Application.Features.Stacks.Commands;

internal static class StackLicenseConfigurationPolicy
{
    public static async ValueTask<Result> EnsureAllowedAsync(
        StackSpec? currentSpec,
        StackDriftPolicy? currentDriftPolicy,
        StackSpec proposedSpec,
        StackDriftPolicy? proposedDriftPolicy,
        ILicenseEntitlementService entitlementService,
        CancellationToken cancellationToken)
    {
        if (ExpandsOperationalGuardrails(
                currentSpec,
                currentDriftPolicy,
                proposedSpec,
                proposedDriftPolicy))
        {
            var guardrails = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken);
            if (guardrails.IsFailure(out var error))
                return Result.Failure(error);
        }

        if (ExpandsAutomatedOperations(currentSpec, proposedSpec))
        {
            var automated = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (automated.IsFailure(out var error))
                return Result.Failure(error);
        }

        return Result.Success();
    }

    internal static bool ExpandsDriftPolicy(
        StackDriftPolicy? current,
        StackDriftPolicy? proposed)
    {
        var existing = (current ?? StackDriftPolicy.Disabled).Normalize();
        var candidate = (proposed ?? StackDriftPolicy.Disabled).Normalize();

        if (candidate.Mode > existing.Mode)
            return true;
        if (candidate.Mode < existing.Mode || candidate.Mode == StackDriftMode.Disabled)
            return false;

        return (!existing.AlertOnDrift && candidate.AlertOnDrift)
            || (!existing.MarkDegraded && candidate.MarkDegraded)
            || (!existing.AutoStartStoppedContainers && candidate.AutoStartStoppedContainers)
            || (!existing.AutoResumePausedContainers && candidate.AutoResumePausedContainers)
            || (!existing.RemoveExtraContainers && candidate.RemoveExtraContainers);
    }

    private static bool ExpandsOperationalGuardrails(
        StackSpec? currentSpec,
        StackDriftPolicy? currentDriftPolicy,
        StackSpec proposedSpec,
        StackDriftPolicy? proposedDriftPolicy)
        => ExpandsDriftPolicy(currentDriftPolicy, proposedDriftPolicy)
            || GetUpdateBehaviorRank(proposedSpec) > GetUpdateBehaviorRank(currentSpec);

    private static bool ExpandsAutomatedOperations(
        StackSpec? currentSpec,
        StackSpec proposedSpec)
    {
        var currentRedeployBindings = (currentSpec?.BuildImageBindings ?? [])
            .Where(static binding => binding.RedeployOnBuild)
            .ToArray();
        var addsRedeployBinding = (proposedSpec.BuildImageBindings ?? [])
            .Where(static binding => binding.RedeployOnBuild)
            .Any(candidate => !currentRedeployBindings.Any(existing =>
                existing.BuildProjectId == candidate.BuildProjectId
                && string.Equals(
                    existing.ServiceName,
                    candidate.ServiceName,
                    StringComparison.OrdinalIgnoreCase)));
        if (addsRedeployBinding)
            return true;

        if (!IsMutatingWebhook(proposedSpec))
            return false;
        if (!IsMutatingWebhook(currentSpec))
            return true;

        return currentSpec is GitStack currentGit
            && proposedSpec is GitStack proposedGit
            && currentGit.Webhook != proposedGit.Webhook;
    }

    private static bool IsMutatingWebhook(StackSpec? spec)
        => spec is GitStack
        {
            Webhook: { Enabled: true },
            UpdateBehavior: StackUpdateBehavior.ServiceAutoDeploy
                or StackUpdateBehavior.StackAutoDeploy
        };

    private static int GetUpdateBehaviorRank(StackSpec? spec)
        => spec switch
        {
            ManualStack { UpdateBehavior: StackUpdateBehavior.ServiceAutoDeploy } => 1,
            GitStack { UpdateBehavior: StackUpdateBehavior.ServiceAutoDeploy } => 1,
            ManualStack { UpdateBehavior: StackUpdateBehavior.StackAutoDeploy } => 2,
            GitStack { UpdateBehavior: StackUpdateBehavior.StackAutoDeploy } => 2,
            _ => 0
        };
}
