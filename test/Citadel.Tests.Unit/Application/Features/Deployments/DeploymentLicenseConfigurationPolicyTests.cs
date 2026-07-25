using Application.Features.Deployments.Commands;
using Domain;
using Domain.Entities.Deployments;

namespace Tests.Unit.Application.Features.Deployments;

public sealed class DeploymentLicenseConfigurationPolicyTests
{
    [Fact]
    public void ExpandsAutomatedOperations_Should_Ignore_Unrelated_Changes()
    {
        var buildProjectId = Guid.CreateVersion7();
        var current = Spec(buildProjectId, redeployOnBuild: true);
        var proposed = current with { Labels = new Dictionary<string, string> { ["env"] = "prod" } };

        Assert.False(
            DeploymentLicenseConfigurationPolicy.ExpandsAutomatedOperations(
                current,
                proposed));
    }

    [Fact]
    public void ExpandsAutomatedOperations_Should_Detect_A_Redeploy_Target_Change()
    {
        var current = Spec(Guid.CreateVersion7(), redeployOnBuild: true);
        var proposed = Spec(Guid.CreateVersion7(), redeployOnBuild: true);

        Assert.True(
            DeploymentLicenseConfigurationPolicy.ExpandsAutomatedOperations(
                current,
                proposed));
    }

    [Fact]
    public void ExpandsAutomatedOperations_Should_Allow_Disabling_Redeploy_On_Build()
    {
        var buildProjectId = Guid.CreateVersion7();
        var current = Spec(buildProjectId, redeployOnBuild: true);
        var proposed = Spec(buildProjectId, redeployOnBuild: false);

        Assert.False(
            DeploymentLicenseConfigurationPolicy.ExpandsAutomatedOperations(
                current,
                proposed));
    }

    private static DeploymentSpec Spec(Guid buildProjectId, bool redeployOnBuild)
        => new(
            new BuildImage(buildProjectId, redeployOnBuild),
            UpdateBehavior.Disabled);
}
