using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;

namespace Application.Features.Stacks;

internal static class SwarmStackConfigurationPolicy
{
    public static IReadOnlyList<SwarmStackCompatibilityIssue> GetIssues(
        StackSpec spec,
        StackDriftPolicy? driftPolicy)
    {
        var issues = new List<SwarmStackCompatibilityIssue>();

        if (spec.DestroyBeforeDeploy)
        {
            issues.Add(Error(
                "stack.destroy_before_deploy",
                "Destroy before deploy is only available for Docker Standalone Stacks.",
                "spec.destroyBeforeDeploy"));
        }

        if (HasCommands(spec.PreDeploy))
        {
            issues.Add(Error(
                "stack.pre_deploy",
                "Pre-deploy commands are only available for Docker Standalone Stacks.",
                "spec.preDeploy"));
        }

        if (HasCommands(spec.PostDeploy))
        {
            issues.Add(Error(
                "stack.post_deploy",
                "Post-deploy commands are only available for Docker Standalone Stacks.",
                "spec.postDeploy"));
        }

        if (driftPolicy is { Mode: not StackDriftMode.Disabled })
        {
            issues.Add(Error(
                "stack.container_drift",
                "Container drift management is only available for Docker Standalone Stacks.",
                "driftPolicy.mode"));
        }

        return issues;
    }

    private static bool HasCommands(StackCommand? command)
        => command?.Commands.Any(static value => !string.IsNullOrWhiteSpace(value)) == true;

    private static SwarmStackCompatibilityIssue Error(string code, string message, string fieldPath)
        => new(SwarmStackCompatibilitySeverity.Error, code, message, fieldPath);
}
