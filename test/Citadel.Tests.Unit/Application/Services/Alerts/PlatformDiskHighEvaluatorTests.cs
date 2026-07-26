using Application.Services.Alerts;
using Domain;
using Domain.Entities.Alerts;

namespace Tests.Unit.Application.Services.Alerts;

public sealed class PlatformDiskHighEvaluatorTests
{
    [Theory]
    [InlineData(69.99, false)]
    [InlineData(70, true)]
    [InlineData(90, true)]
    public void Evaluate_ShouldApplyInclusiveThreshold(double usage, bool expectedMatch)
    {
        var evaluator = new PlatformDiskHighEvaluator();
        var rule = CreateRule(70);
        var snapshot = new PlatformAlertSnapshot(
            Guid.CreateVersion7(),
            "docker-01",
            CpuUsage: 0,
            RamUsage: 0,
            AgentVersion: "1.0.0",
            DiskUsage: usage,
            DiskUsedBytes: 70,
            DiskTotalBytes: 100);

        var match = Assert.Single(evaluator.Evaluate(rule, CreateContext(snapshot)));

        Assert.Equal(expectedMatch, match.IsMatch);
        if (expectedMatch)
            Assert.IsType<PlatformDiskHighAlertInfo>(match.Info);
    }

    [Fact]
    public void Evaluate_ShouldReturnNonMatchWhenDiskSampleIsIncomplete()
    {
        var evaluator = new PlatformDiskHighEvaluator();
        var snapshot = new PlatformAlertSnapshot(
            Guid.CreateVersion7(),
            "docker-01",
            CpuUsage: 0,
            RamUsage: 0,
            AgentVersion: "1.0.0",
            DiskUsage: null,
            DiskUsedBytes: null,
            DiskTotalBytes: null);

        var match = Assert.Single(evaluator.Evaluate(CreateRule(70), CreateContext(snapshot)));

        Assert.False(match.IsMatch);
    }

    [Theory]
    [InlineData(-0.01)]
    [InlineData(100.01)]
    [InlineData(double.NaN)]
    [InlineData(double.PositiveInfinity)]
    public void Evaluate_ShouldReturnNonMatchWhenDiskPercentageIsInvalid(double usage)
    {
        var evaluator = new PlatformDiskHighEvaluator();
        var snapshot = new PlatformAlertSnapshot(
            Guid.CreateVersion7(),
            "docker-01",
            CpuUsage: 0,
            RamUsage: 0,
            AgentVersion: "1.0.0",
            DiskUsage: usage,
            DiskUsedBytes: 70,
            DiskTotalBytes: 100);

        var match = Assert.Single(evaluator.Evaluate(CreateRule(70), CreateContext(snapshot)));

        Assert.False(match.IsMatch);
    }

    private static AlertRule CreateRule(double threshold)
        => new(
            name: "Disk high",
            description: "Disk threshold test",
            type: AlertType.PlatformDiskHigh,
            severity: AlertSeverity.Warning,
            cooldownSeconds: 300,
            status: AlertRuleStatus.Enabled,
            createdByActorId: Guid.CreateVersion7(),
            requiredMatches: 3,
            threshold: threshold);

    private static AlertEvaluationContext CreateContext(PlatformAlertSnapshot snapshot)
        => new(
            DateTime.UtcNow,
            Platforms: [snapshot],
            Deployments: [],
            Stacks: []);
}
