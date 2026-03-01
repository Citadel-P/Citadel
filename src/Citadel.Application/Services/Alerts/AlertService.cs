using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Microsoft.Extensions.Logging;

namespace Application.Services.Alerts;

internal interface IAlertService
{
    Task ProcessAsync(AlertType type, AlertEvaluationContext context, CancellationToken ct);
}

public sealed class AlertService(IEnumerable<IAlertEvaluator> evaluators, IAlertRuleProvider alertRuleProvider, IDbWorkQueue dbQueue,
    ILogger<AlertService> logger) : IAlertService
{
    private readonly Dictionary<AlertType, IAlertEvaluator> _evaluators = evaluators.ToDictionary(x => x.Type);

    public async Task ProcessAsync(AlertType type, AlertEvaluationContext context, CancellationToken ct)
    {
        // Most-severe-wins: per resource we keep only the highest-severity rule
        var candidates = new Dictionary<Guid, (AlertRule Rule, List<AlertMatch> Matches)>();

        foreach (var rule in alertRuleProvider.Current.Get(type))
        {
            if (!_evaluators.TryGetValue(rule.Type, out var evaluator))
                throw new InvalidOperationException($"No evaluator registered for {rule.Type}");

            var matches = evaluator.Evaluate(rule, context);

            foreach (var match in ApplyScope(rule, matches))
            {
                if (!candidates.TryGetValue(match.ResourceId, out var existing))
                {
                    candidates[match.ResourceId] = (rule, [match]);
                }
                else if (rule.Severity > existing.Rule.Severity)
                {
                    candidates[match.ResourceId] = (rule, [match]);
                }
                else if (rule.Id == existing.Rule.Id)
                {
                    existing.Matches.Add(match);
                }
            }
        }

        foreach (var (_, (rule, matches)) in candidates)
        {
            foreach (var match in matches)
            {
                var workItem = new AlertStateWorkItem(
                    rule,
                    match,
                    context.UtcNow,
                    logger);

                await dbQueue.EnqueueAsync(workItem, ct);
            }
        }
    }

    private static IEnumerable<AlertMatch> ApplyScope(AlertRule rule, IEnumerable<AlertMatch> matches)
    {
        if (rule.LimitedTo.Count == 0)
            return matches;

        var allowed = rule.LimitedTo
            .Select(x => x.ResourceId)
            .ToHashSet();

        return matches.Where(m => allowed.Contains(m.ResourceId));
    }
}

internal sealed class AlertStateWorkItem(AlertRule rule, AlertMatch match, DateTime utcNow, ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken token)
    {
        try
        {
            var existingState = await uow.AlertRules.GetStateAsync(
                rule.Id,
                match.ResourceId,
                token);
            var state = existingState
                ?? new AlertRuleState(
                    rule.Id,
                    match.ResourceId,
                    rule.CreatedByActorId,
                    0);

            // Threshold alerts (e.g. CPU/RAM)
            if (AlertTypeMetadata.IsThreshold(rule.Type))
            {
                if (!rule.CanTrigger(utcNow, state))
                {
                    await uow.AlertRules.UpsertAlertRuleStateAsync(state, token);
                    await uow.CommitAsync(token);
                    return;
                }

                state.RegisterMatch();

                if (!state.CanFire(rule) || !rule.CanTrigger(utcNow, state))
                {
                    await uow.AlertRules.UpsertAlertRuleStateAsync(state, token);
                    await uow.CommitAsync(token);
                    return;
                }
            }
            // Event alerts (DeploymentAutoUpdated, etc.)
            else
            {
                if (!rule.CanTrigger(utcNow, state))
                {
                    await uow.AlertRules.UpsertAlertRuleStateAsync(state, token);
                    await uow.CommitAsync(token);
                    return;
                }
            }

            var evt = new AlertEvent(
                rule.Id,
                rule.Type,
                rule.Severity,
                match.Info!,
                match.ResourceId,
                match.ResourceType);

            state.MarkTriggered(utcNow);

            if (AlertTypeMetadata.IsThreshold(rule.Type))
                state.Reset();

            await uow.AlertRules.UpsertAlertRuleStateAsync(state, token);
            await uow.AlertEvents.AddAsync(evt, token);
            // TODO: send to client (Shoutrrr & signalr)

            await uow.CommitAsync(token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Alert processing DB work failed.");
        }
    }
}