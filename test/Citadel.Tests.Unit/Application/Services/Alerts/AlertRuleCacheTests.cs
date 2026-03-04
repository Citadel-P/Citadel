using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Services.Alerts;

public class AlertRuleCacheTests
{
    [Fact]
    public void Upsert_AddsRuleToSnapshot()
    {
        var cache = CreateCache();
        var rule = CreateRule(Guid.NewGuid(), AlertType.PlatformUnreachable);

        cache.Upsert(rule);

        var current = cache.Current.Get(AlertType.PlatformUnreachable);
        Assert.Single(current);
        Assert.Equal(rule.Id, current[0].Id);
    }

    [Fact]
    public void Upsert_ReplacesExistingRule_WithSameId()
    {
        var id = Guid.NewGuid();
        var cache = CreateCache();

        var first = CreateRule(id, AlertType.PlatformUnreachable, AlertSeverity.Warning);
        var updated = CreateRule(id, AlertType.PlatformUnreachable, AlertSeverity.Critical);

        cache.Upsert(first);
        cache.Upsert(updated);

        var current = cache.Current.Get(AlertType.PlatformUnreachable);
        Assert.Single(current);
        Assert.Equal(AlertSeverity.Critical, current[0].Severity);
    }

    [Fact]
    public void Upsert_MovesRule_WhenTypeChanges()
    {
        var id = Guid.NewGuid();
        var cache = CreateCache();

        var first = CreateRule(id, AlertType.PlatformUnreachable);
        var moved = CreateRule(id, AlertType.DeploymentAutoUpdated);

        cache.Upsert(first);
        cache.Upsert(moved);

        Assert.Empty(cache.Current.Get(AlertType.PlatformUnreachable));
        var target = cache.Current.Get(AlertType.DeploymentAutoUpdated);
        Assert.Single(target);
        Assert.Equal(id, target[0].Id);
    }

    [Fact]
    public void Remove_DeletesRulesByIds()
    {
        var cache = CreateCache();
        var id1 = Guid.NewGuid();
        var id2 = Guid.NewGuid();

        cache.Upsert(CreateRule(id1, AlertType.PlatformUnreachable));
        cache.Upsert(CreateRule(id2, AlertType.PlatformUnreachable));

        cache.Remove([id1]);

        var current = cache.Current.Get(AlertType.PlatformUnreachable);
        Assert.Single(current);
        Assert.Equal(id2, current[0].Id);
    }

    [Fact]
    public async Task ReloadAsync_LoadsRulesFromRepository()
    {
        var id1 = Guid.NewGuid();
        var id2 = Guid.NewGuid();
        var rules = new[]
        {
            CreateRule(id1, AlertType.PlatformUnreachable),
            CreateRule(id2, AlertType.DeploymentAutoUpdated)
        };

        var repoMock = new Mock<IAlertRuleRepository>();
        repoMock
            .Setup(x => x.GetAllAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(rules);

        var uowMock = new Mock<IUnitOfWork>();
        uowMock.SetupGet(x => x.AlertRules).Returns(repoMock.Object);

        await using var provider = new ServiceCollection()
            .AddSingleton(uowMock.Object)
            .BuildServiceProvider();

        var scopeFactory = provider.GetRequiredService<IServiceScopeFactory>();
        var cache = new AlertRuleCache(scopeFactory, NullLogger<AlertRuleCache>.Instance);

        await cache.ReloadAsync(TestContext.Current.CancellationToken);

        Assert.Single(cache.Current.Get(AlertType.PlatformUnreachable));
        Assert.Single(cache.Current.Get(AlertType.DeploymentAutoUpdated));
    }

    private static AlertRuleCache CreateCache()
    {
        var provider = new ServiceCollection().BuildServiceProvider();
        var scopeFactory = provider.GetRequiredService<IServiceScopeFactory>();
        return new AlertRuleCache(scopeFactory, NullLogger<AlertRuleCache>.Instance);
    }

    private static AlertRule CreateRule(Guid id, AlertType type, AlertSeverity severity = AlertSeverity.Warning)
    {
        int? requiredMatches = AlertTypeMetadata.IsThreshold(type) ? 1 : null;
        double? threshold = AlertTypeMetadata.IsThreshold(type) ? 50d : null;

        return AlertRule.FromPersistence(
            id: id,
            type: type,
            severity: severity,
            cooldownSeconds: 300,
            isEnabled: true,
            createdByActorId: Guid.NewGuid(),
            createdAt: DateTime.UtcNow,
            requiredMatches: requiredMatches,
            threshold: threshold,
            channelIds: [],
            limitedTo: [],
            quietHours: []);
    }
}
