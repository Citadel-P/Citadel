using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
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

public sealed class AlertRuleCache(IDbWorkQueue dbQueue, ILogger<AlertRuleCache> logger) : IAlertRuleProvider
{
    private volatile AlertRuleSnapshot _snapshot = Empty();

    public AlertRuleSnapshot Current => _snapshot;

    public async Task ReloadAsync(CancellationToken ct = default)
    {
        try
        {
            var tcs = new TaskCompletionSource<IEnumerable<AlertRule>>();

            await dbQueue.EnqueueAsync(new LoadAlertRulesWorkItem(tcs), ct);

            var rules = await tcs.Task;

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

    private static AlertRuleSnapshot Empty() => new([]);
}

internal sealed class LoadAlertRulesWorkItem(TaskCompletionSource<IEnumerable<AlertRule>> tcs) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken token)
    {
        try
        {
            var rules = await uow.Alerters.GetAllAsync(token);
            tcs.SetResult(rules);
        }
        catch (Exception ex)
        {
            tcs.SetException(ex);
        }
    }
}