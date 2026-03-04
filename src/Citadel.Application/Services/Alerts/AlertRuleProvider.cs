using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Collections.Immutable;

namespace Application.Services.Alerts;

public interface IAlertRuleProvider
{
    AlertRuleSnapshot Current { get; }
}

public sealed class AlertRuleSnapshot(
    IReadOnlyDictionary<AlertType, IReadOnlyList<AlertRule>> byType,
    IReadOnlyDictionary<Guid, AlertChannel> channels)
{
    public IReadOnlyDictionary<AlertType, IReadOnlyList<AlertRule>> ByType { get; } = byType;
    public IReadOnlyDictionary<Guid, AlertChannel> Channels { get; } = channels;

    public IReadOnlyList<AlertRule> Get(AlertType type)
        => ByType.TryGetValue(type, out var rules) ? rules : [];

    public bool TryGetChannel(Guid id, out AlertChannel channel)
        => Channels.TryGetValue(id, out channel!);

    public static AlertRuleSnapshot Empty { get; } =
        new(ImmutableDictionary<AlertType, IReadOnlyList<AlertRule>>.Empty,
            ImmutableDictionary<Guid, AlertChannel>.Empty);
}

public sealed class AlertRuleCache(
    IServiceScopeFactory scopeFactory,
    ILogger<AlertRuleCache> logger) : IAlertRuleProvider
{
    private readonly ConcurrentDictionary<AlertType, ConcurrentDictionary<Guid, AlertRule>> _rulesByType = new();
    private readonly ConcurrentDictionary<Guid, AlertType> _ruleTypeIndex = new();
    private readonly ConcurrentDictionary<Guid, AlertChannel> _channels = new();

    private AlertRuleSnapshot _snapshot = AlertRuleSnapshot.Empty;
    public AlertRuleSnapshot Current => _snapshot;

    public async Task ReloadAsync(CancellationToken ct = default)
    {
        try
        {
            var (rules, channels) = await LoadData(ct);
            RebuildStore(rules, channels);
            PublishSnapshot();

            logger.LogInformation(
                "AlertRuleCache reloaded with {RuleCount} rules and {ChannelCount} channels",
                rules.Count,
                channels.Count);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to reload alert rule cache");
        }
    }

    public void Upsert(AlertRule rule)
    {
        if (_ruleTypeIndex.TryGetValue(rule.Id, out var previousType) &&
            previousType != rule.Type &&
            _rulesByType.TryGetValue(previousType, out var oldBucket))
        {
            oldBucket.TryRemove(rule.Id, out _);
            if (oldBucket.IsEmpty)
                _rulesByType.TryRemove(previousType, out _);
        }

        var bucket = _rulesByType.GetOrAdd(rule.Type, _ => new());
        bucket[rule.Id] = rule;
        _ruleTypeIndex[rule.Id] = rule.Type;

        PublishSnapshot();
    }

    public void Remove(IEnumerable<Guid> ids)
    {
        var list = ids as ICollection<Guid> ?? ids.ToList();
        if (list.Count == 0)
            return;

        foreach (var id in list)
        {
            if (!_ruleTypeIndex.TryRemove(id, out var type))
                continue;

            if (_rulesByType.TryGetValue(type, out var bucket))
            {
                bucket.TryRemove(id, out _);
                if (bucket.IsEmpty)
                    _rulesByType.TryRemove(type, out _);
            }
        }

        PublishSnapshot();
    }

    public void UpsertChannel(AlertChannel channel)
    {
        _channels[channel.Id] = channel;
        PublishSnapshot();
    }

    public void RemoveChannels(IEnumerable<Guid> ids)
    {
        var list = ids as ICollection<Guid> ?? ids.ToList();
        if (list.Count == 0)
            return;

        foreach (var id in list)
            _channels.TryRemove(id, out _);

        PublishSnapshot();
    }

    private void RebuildStore(
        IReadOnlyCollection<AlertRule> rules,
        IReadOnlyCollection<AlertChannel> channels)
    {
        _rulesByType.Clear();
        _ruleTypeIndex.Clear();
        _channels.Clear();

        foreach (var rule in rules)
        {
            var bucket = _rulesByType.GetOrAdd(rule.Type, _ => new());
            bucket[rule.Id] = rule;
            _ruleTypeIndex[rule.Id] = rule.Type;
        }

        foreach (var channel in channels)
            _channels[channel.Id] = channel;
    }

    private void PublishSnapshot()
    {
        var rulesSnapshot = new Dictionary<AlertType, IReadOnlyList<AlertRule>>(
            _rulesByType.Count);

        foreach (var (type, bucket) in _rulesByType)
            rulesSnapshot[type] = bucket.Values.ToArray();

        var channelsSnapshot = _channels.ToImmutableDictionary();

        var snapshot = new AlertRuleSnapshot(rulesSnapshot, channelsSnapshot);
        Interlocked.Exchange(ref _snapshot, snapshot);
    }

    private async Task<(IReadOnlyCollection<AlertRule> Rules, IReadOnlyCollection<AlertChannel> Channels)>
        LoadData(CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var rules = await uow.AlertRules.GetAllAsync(ct);
        var channels = await uow.AlertRules.GetAllChannelsAsync(ct);

        return ([.. rules], [.. channels]);
    }
}