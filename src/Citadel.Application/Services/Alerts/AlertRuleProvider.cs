using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
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
    private readonly Dictionary<AlertType, Dictionary<Guid, AlertRule>> _rulesByType = [];
    private readonly Dictionary<Guid, AlertType> _ruleTypeIndex = [];
    private readonly Dictionary<Guid, AlertChannel> _channels = [];
    private readonly Lock _gate = new();
    private long _mutationVersion;

    private AlertRuleSnapshot _snapshot = AlertRuleSnapshot.Empty;
    public AlertRuleSnapshot Current => Volatile.Read(ref _snapshot);

    public async Task ReloadAsync(CancellationToken ct = default)
    {
        try
        {
            while (true)
            {
                var observedVersion = Volatile.Read(ref _mutationVersion);
                var (rules, channels) = await LoadData(ct);

                using (_gate.EnterScope())
                {
                    if (observedVersion != _mutationVersion)
                        continue;

                    RebuildStore(rules, channels);
                    PublishSnapshot();
                }

                logger.LogInformation(
                    "AlertRuleCache reloaded with {RuleCount} rules and {ChannelCount} channels",
                    rules.Count,
                    channels.Count);
                return;
            }
        }
        catch (OperationCanceledException) when (ct.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to reload alert rule cache");
        }
    }

    public void Upsert(AlertRule rule)
    {
        using (_gate.EnterScope())
        {
            if (_ruleTypeIndex.TryGetValue(rule.Id, out var previousType) &&
                previousType != rule.Type &&
                _rulesByType.TryGetValue(previousType, out var oldBucket))
            {
                oldBucket.Remove(rule.Id);
                if (oldBucket.Count == 0)
                    _rulesByType.Remove(previousType);
            }

            if (!_rulesByType.TryGetValue(rule.Type, out var bucket))
                _rulesByType[rule.Type] = bucket = [];

            bucket[rule.Id] = rule;
            _ruleTypeIndex[rule.Id] = rule.Type;
            _mutationVersion++;
            PublishSnapshot();
        }
    }

    public void Remove(IEnumerable<Guid> ids)
    {
        var list = ids as ICollection<Guid> ?? ids.ToList();
        if (list.Count == 0)
            return;

        using (_gate.EnterScope())
        {
            var changed = false;
            foreach (var id in list)
            {
                if (!_ruleTypeIndex.Remove(id, out var type))
                    continue;

                changed = true;
                if (_rulesByType.TryGetValue(type, out var bucket))
                {
                    bucket.Remove(id);
                    if (bucket.Count == 0)
                        _rulesByType.Remove(type);
                }
            }

            _mutationVersion++;
            if (changed)
                PublishSnapshot();
        }
    }

    public void UpsertChannel(AlertChannel channel)
    {
        using (_gate.EnterScope())
        {
            _channels[channel.Id] = channel;
            _mutationVersion++;
            PublishSnapshot();
        }
    }

    public void RemoveChannels(IEnumerable<Guid> ids)
    {
        var list = ids as ICollection<Guid> ?? ids.ToList();
        if (list.Count == 0)
            return;

        using (_gate.EnterScope())
        {
            var changed = false;
            foreach (var id in list)
                changed |= _channels.Remove(id);

            _mutationVersion++;
            if (changed)
                PublishSnapshot();
        }
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
            if (!_rulesByType.TryGetValue(rule.Type, out var bucket))
                _rulesByType[rule.Type] = bucket = [];

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
