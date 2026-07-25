using Domain;
using Domain.Entities.Deployments;

namespace Application.Features.Deployments.Commands;

internal static class DeploymentLicenseConfigurationPolicy
{
    public static bool ExpandsOperationalGuardrails(
        DeploymentSpec? current,
        DeploymentSpec proposed)
        => proposed.UpdateBehavior == UpdateBehavior.AutoDeploy
            && current?.UpdateBehavior != UpdateBehavior.AutoDeploy;

    public static bool ExpandsAutomatedOperations(
        DeploymentSpec? current,
        DeploymentSpec proposed)
    {
        if (proposed.Image is not BuildImage { RedeployOnBuild: true } proposedBuild)
            return false;

        var currentBuild = current?.Image as BuildImage;
        return currentBuild is not { RedeployOnBuild: true }
            || currentBuild.BuildProjectId != proposedBuild.BuildProjectId;
    }
}
