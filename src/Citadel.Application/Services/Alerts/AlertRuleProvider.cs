using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;

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
    private readonly ConcurrentDictionary<AlertType, ConcurrentDictionary<Guid, AlertRule>> _rulesByType = new();
    private readonly ConcurrentDictionary<Guid, AlertType> _ruleTypeIndex = new();
    private volatile AlertRuleSnapshot _snapshot = Empty();

    public AlertRuleSnapshot Current => _snapshot;

    public async Task ReloadAsync(CancellationToken ct = default)
    {
        try
        {
            var rules = await GetRules(ct);
            RebuildStore(rules);
            PublishSnapshot();

            logger.LogInformation("AlertRuleCache reloaded with {Count} rules", rules.Count());
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to reload alert rule cache");
        }
    }

    public void Upsert(AlertRule rule)
    {
        if (_ruleTypeIndex.TryGetValue(rule.Id, out var previousType) && previousType != rule.Type)
        {
            if (_rulesByType.TryGetValue(previousType, out var previousBucket))
            {
                previousBucket.TryRemove(rule.Id, out _);

                if (previousBucket.IsEmpty)
                {
                    _rulesByType.TryRemove(previousType, out _);
                }
            }
        }

        var targetBucket = _rulesByType.GetOrAdd(rule.Type, _ => new ConcurrentDictionary<Guid, AlertRule>());
        targetBucket[rule.Id] = rule;
        _ruleTypeIndex[rule.Id] = rule.Type;

        PublishSnapshot();
    }

    public void Remove(IEnumerable<Guid> ids)
    {
        var toRemove = ids as ICollection<Guid> ?? ids.ToList();
        if (toRemove.Count == 0)
        {
            return;
        }

        foreach (var id in toRemove)
        {
            if (!_ruleTypeIndex.TryRemove(id, out var type))
            {
                continue;
            }

            if (_rulesByType.TryGetValue(type, out var bucket))
            {
                bucket.TryRemove(id, out _);

                if (bucket.IsEmpty)
                {
                    _rulesByType.TryRemove(type, out _);
                }
            }
        }

        PublishSnapshot();
    }

    private static AlertRuleSnapshot BuildSnapshot(IEnumerable<AlertRule> rules)
    {
        var dict = rules
            .GroupBy(r => r.Type)
            .ToDictionary(
                g => g.Key,
                g => (IReadOnlyList<AlertRule>)[.. g]);

        return new AlertRuleSnapshot(dict);
    }

    private void RebuildStore(IEnumerable<AlertRule> rules)
    {
        _rulesByType.Clear();
        _ruleTypeIndex.Clear();
        foreach (var rule in rules)
        {
            var bucket = _rulesByType.GetOrAdd(rule.Type, _ => new ConcurrentDictionary<Guid, AlertRule>());
            bucket[rule.Id] = rule;
            _ruleTypeIndex[rule.Id] = rule.Type;
        }
    }

    private void PublishSnapshot()
    {
        var snapshotRules = _rulesByType
            .SelectMany(x => x.Value.Values)
            .ToArray();

        _snapshot = BuildSnapshot(snapshotRules);
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
