using Application.Services.Alerts;
using Domain;
using Domain.Entities.Alerts;

namespace Tests.Unit.Application.Services.Alerts;

public sealed class ConfigurationResolutionAlertEvaluatorTests
{
    [Fact]
    public void StackConfigurationResolutionFailedEvaluator_Should_Emit_Stack_Alert()
    {
        var stackId = Guid.CreateVersion7();
        var reason = "Secret API_KEY could not be decrypted.";
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            StackConfigurationFailures:
            [
                new StackConfigurationResolutionFailureAlertSnapshot(stackId, "api", reason)
            ]);

        var match = Assert.Single(new StackConfigurationResolutionFailedEvaluator().Evaluate(CreateRule(AlertType.StackConfigurationResolutionFailed), context));

        Assert.True(match.IsMatch);
        Assert.Equal(stackId, match.ResourceId);
        Assert.Equal("api", match.ResourceName);
        Assert.Equal(AlertResourceType.Stack, match.ResourceType);
        Assert.Equal(reason, match.DeduplicationComponent);
        var info = Assert.IsType<StackConfigurationResolutionFailedAlertInfo>(match.Info);
        Assert.Equal("api", info.StackName);
        Assert.Equal(reason, info.Reason);
        Assert.True(AlertTypeMetadata.IsValidInfo(AlertType.StackConfigurationResolutionFailed, info));
    }

    [Fact]
    public void DeploymentConfigurationResolutionFailedEvaluator_Should_Emit_Deployment_Alert()
    {
        var deploymentId = Guid.CreateVersion7();
        var reason = "Secret provider token for API_TOKEN could not be decrypted.";
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            DeploymentConfigurationFailures:
            [
                new DeploymentConfigurationResolutionFailureAlertSnapshot(deploymentId, "worker", reason)
            ]);

        var match = Assert.Single(new DeploymentConfigurationResolutionFailedEvaluator().Evaluate(CreateRule(AlertType.DeploymentConfigurationResolutionFailed), context));

        Assert.True(match.IsMatch);
        Assert.Equal(deploymentId, match.ResourceId);
        Assert.Equal("worker", match.ResourceName);
        Assert.Equal(AlertResourceType.Deployment, match.ResourceType);
        Assert.Equal(reason, match.DeduplicationComponent);
        var info = Assert.IsType<DeploymentConfigurationResolutionFailedAlertInfo>(match.Info);
        Assert.Equal("worker", info.DeploymentName);
        Assert.Equal(reason, info.Reason);
        Assert.True(AlertTypeMetadata.IsValidInfo(AlertType.DeploymentConfigurationResolutionFailed, info));
    }

    [Fact]
    public void AutomationActionRunFailedEvaluator_Should_Emit_Automation_Action_Alert()
    {
        var actionId = Guid.CreateVersion7();
        var runId = Guid.CreateVersion7();
        var reason = "Deno exited with code 7.";
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            AutomationActionRunFailures:
            [
                new AutomationActionRunFailureAlertSnapshot(
                    actionId,
                    "prune-images",
                    runId,
                    ActionRunTrigger.Schedule,
                    ActionRunStatus.Failed,
                    7,
                    1250,
                    reason)
            ]);

        var match = Assert.Single(new AutomationActionRunFailedEvaluator().Evaluate(CreateRule(AlertType.AutomationActionRunFailed), context));

        Assert.True(match.IsMatch);
        Assert.Equal(actionId, match.ResourceId);
        Assert.Equal("prune-images", match.ResourceName);
        Assert.Equal(AlertResourceType.AutomationAction, match.ResourceType);
        Assert.Equal(runId.ToString("N"), match.DeduplicationComponent);
        var info = Assert.IsType<AutomationActionRunFailedAlertInfo>(match.Info);
        Assert.Equal("prune-images", info.ActionName);
        Assert.Equal(runId, info.RunId);
        Assert.Equal(reason, info.Reason);
        Assert.True(AlertTypeMetadata.IsValidInfo(AlertType.AutomationActionRunFailed, info));
    }

    private static AlertRule CreateRule(AlertType type)
        => new(
            name: type.ToString(),
            description: null,
            type: type,
            severity: AlertSeverity.Warning,
            cooldownSeconds: 300,
            status: AlertRuleStatus.Enabled,
            createdByActorId: Guid.CreateVersion7());
}
