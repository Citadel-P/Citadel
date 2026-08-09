using Application.Services.Alerts;
using Domain;
using Domain.Entities.Alerts;
using Hosting.Common;

namespace Tests.Unit.Application.Services.Alerts;

public sealed class SwarmServiceOperationFailedEvaluatorTests
{
    [Fact]
    public void Evaluate_ShouldCreateServiceScopedMatchForFailedOperation()
    {
        var serviceId = Guid.CreateVersion7();
        var operationId = Guid.CreateVersion7();
        var rule = new AlertRule(
            "Swarm Service operation failed",
            null,
            AlertType.SwarmServiceOperationFailed,
            AlertSeverity.Critical,
            null,
            AlertRuleStatus.Enabled,
            Constants.SystemId);
        var context = new AlertEvaluationContext(
            DateTime.UtcNow,
            [],
            [],
            [],
            SwarmServiceOperationFailures:
            [
                new SwarmServiceOperationFailureAlertSnapshot(
                    serviceId,
                    "redis",
                    operationId,
                    SwarmServiceOperationKind.Apply,
                    "rollout paused")
            ]);

        var match = Assert.Single(new SwarmServiceOperationFailedEvaluator().Evaluate(rule, context));

        Assert.Equal(serviceId, match.ResourceId);
        Assert.Equal(AlertResourceType.SwarmService, match.ResourceType);
        Assert.Equal(operationId.ToString("N"), match.DeduplicationComponent);
        var info = Assert.IsType<SwarmServiceOperationFailedAlertInfo>(match.Info);
        Assert.Equal("redis", info.ServiceName);
        Assert.Equal("rollout paused", info.Reason);
    }
}
