using Application.Services.Alerts;
using Domain;
using Domain.Entities.Alerts;

namespace Tests.Unit.Application.Services.Alerts;

public sealed class LicenseAlertEvaluatorTests
{
    [Fact]
    public void LicenseEnteredGracePeriodEvaluator_Should_Emit_License_Alert()
    {
        var instanceId = Guid.CreateVersion7();
        var expiresAt = DateTimeOffset.UtcNow.AddMinutes(-5);
        var graceUntil = DateTimeOffset.UtcNow.AddDays(14);
        var context = CreateContext(new LicenseAlertSnapshot(
            instanceId,
            "lic_123",
            "Example Corp",
            "sha256:abc",
            LicenseStatus.GracePeriod,
            expiresAt,
            graceUntil));

        var match = Assert.Single(new LicenseEnteredGracePeriodEvaluator().Evaluate(CreateRule(AlertType.LicenseEnteredGracePeriod), context));

        Assert.True(match.IsMatch);
        Assert.Equal(instanceId, match.ResourceId);
        Assert.Equal("License", match.ResourceName);
        Assert.Equal(AlertResourceType.License, match.ResourceType);
        Assert.Equal("sha256:abc", match.DeduplicationComponent);
        var info = Assert.IsType<LicenseEnteredGracePeriodAlertInfo>(match.Info);
        Assert.Equal("lic_123", info.LicenseId);
        Assert.Equal(expiresAt, info.ExpiresAt);
        Assert.Equal(graceUntil, info.GraceUntil);
        Assert.True(AlertTypeMetadata.IsValidInfo(AlertType.LicenseEnteredGracePeriod, info));
    }

    [Fact]
    public void LicenseExpiredEvaluator_Should_Emit_License_Alert()
    {
        var instanceId = Guid.CreateVersion7();
        var expiresAt = DateTimeOffset.UtcNow.AddDays(-15);
        var graceUntil = DateTimeOffset.UtcNow.AddDays(-1);
        var context = CreateContext(new LicenseAlertSnapshot(
            instanceId,
            "lic_123",
            "Example Corp",
            "sha256:abc",
            LicenseStatus.Expired,
            expiresAt,
            graceUntil));

        var match = Assert.Single(new LicenseExpiredEvaluator().Evaluate(CreateRule(AlertType.LicenseExpired), context));

        Assert.True(match.IsMatch);
        Assert.Equal(instanceId, match.ResourceId);
        Assert.Equal(AlertResourceType.License, match.ResourceType);
        Assert.Equal("sha256:abc", match.DeduplicationComponent);
        var info = Assert.IsType<LicenseExpiredAlertInfo>(match.Info);
        Assert.Equal("lic_123", info.LicenseId);
        Assert.Equal(expiresAt, info.ExpiresAt);
        Assert.Equal(graceUntil, info.GraceUntil);
        Assert.True(AlertTypeMetadata.IsValidInfo(AlertType.LicenseExpired, info));
    }

    [Fact]
    public void LicenseExpiredEvaluator_Should_Not_Emit_When_License_Is_Valid()
    {
        var context = CreateContext(new LicenseAlertSnapshot(
            Guid.CreateVersion7(),
            "lic_123",
            "Example Corp",
            "sha256:abc",
            LicenseStatus.Valid,
            DateTimeOffset.UtcNow.AddDays(30),
            null));

        Assert.Empty(new LicenseExpiredEvaluator().Evaluate(CreateRule(AlertType.LicenseExpired), context));
    }

    private static AlertEvaluationContext CreateContext(LicenseAlertSnapshot license)
        => new(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            Licenses: [license]);

    private static AlertRule CreateRule(AlertType type)
        => new(
            name: type.ToString(),
            description: null,
            type: type,
            severity: AlertSeverity.Warning,
            cooldownSeconds: null,
            status: AlertRuleStatus.Enabled,
            createdByActorId: Guid.CreateVersion7());
}
