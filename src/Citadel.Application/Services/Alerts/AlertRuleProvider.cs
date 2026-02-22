using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Application.Services.Alerts;

public interface IAlertRuleProvider
{
    AlertRuleSnapshot Current { get; }
}

public sealed class AlertRuleSnapshot(Dictionary<AlertType, IReadOnlyList<AlertRule>> byResourceType)
{
    public IReadOnlyDictionary<AlertType, IReadOnlyList<AlertRule>> ByResourceType { get; private set; } = byResourceType;

    public IReadOnlyList<AlertRule> Get(AlertType type)
        => ByResourceType.TryGetValue(type, out var rules) ? rules : [];
}

public sealed class AlertRuleCache(IServiceScopeFactory scopeFactory, ILogger<AlertRuleCache> logger) : IAlertRuleProvider
{
    private volatile AlertRuleSnapshot _snapshot = Empty();

    public AlertRuleSnapshot Current => _snapshot;

    public async Task ReloadAsync(CancellationToken ct = default)
    {
        try
        {
            var rules = await GetRules(ct);
            _snapshot = BuildSnapshot(rules);

            logger.LogInformation("AlertRuleCache reloaded with {Count} rules", rules.Count());
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to reload alert rule cache");
        }
    }

    private static AlertRuleSnapshot BuildSnapshot(IEnumerable<AlertRule> rules)
    {
        var dict = rules
            .GroupBy(r => r.Type)
            .ToDictionary(
                g => g.Key,
                g => (IReadOnlyList<AlertRule>)[.. g]);

        return new AlertRuleSnapshot (dict);
    }

    private async Task<IEnumerable<AlertRule>> GetRules(CancellationToken token)
    {
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            return await uow.AlertRules.GetAllAsync(token);
        }
    }

    private static AlertRuleSnapshot Empty() => new([]);
}
