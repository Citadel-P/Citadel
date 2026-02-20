using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.Extensions.Logging;

namespace Application.Services.Alerts;

internal interface IAlertService
{
    Task<IReadOnlyCollection<AlertEvent>> EvaluateAsync(AlertType type, AlertEvaluationContext context, CancellationToken ct);
}

public sealed class AlertService(
    IEnumerable<IAlertEvaluator> evaluators,
    IAlertRuleProvider alertRuleProvider,
    IDbWorkQueue dbQueue,
    ILogger<AlertService> logger) : IAlertService
{
    private readonly Dictionary<AlertType, IAlertEvaluator> _evaluators = evaluators.ToDictionary(x => x.Type);

    public async Task<IReadOnlyCollection<AlertEvent>> EvaluateAsync(AlertType type, AlertEvaluationContext context, CancellationToken ct)
    {
        var result = new List<AlertEvent>();

        foreach (var rule in alertRuleProvider.Current.Get(type))
        {
            if (!_evaluators.TryGetValue(rule.Type, out var evaluator))
                throw new InvalidOperationException($"No evaluator registered for {rule.Type}");

            var matches = evaluator.Evaluate(rule, context);

            foreach (var match in ApplyScope(rule, matches))
            {
                if (match.ResourceId is null)
                    continue;

                var workItem = new AlertStateWorkItem(
                    rule,
                    match,
                    context.UtcNow,
                    result,
                    logger);

                await dbQueue.EnqueueAsync(workItem, ct);
            }
        }

        return result;
    }

    private static IEnumerable<AlertMatch> ApplyScope(AlertRule rule, IEnumerable<AlertMatch> matches)
    {
        if (rule.Scope == AlertScope.All)
            return matches;

        var allowed = rule.LimitedTo
            .Select(x => x.ResourceId)
            .ToHashSet();

        return matches.Where(m =>
            m.ResourceId is not null &&
            allowed.Contains(m.ResourceId.Value));
    }
}

internal sealed class AlertStateWorkItem(
    AlertRule rule,
    AlertMatch match,
    DateTime utcNow,
    List<AlertEvent> resultSink,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken token)
    {
        try
        {
            var state = await uow.Alerters.GetStateAsync(
                rule.Id,
                match.ResourceId!.Value,
                token)
                ?? new AlertRuleState(
                    rule.Id,
                    match.ResourceId!.Value,
                    rule.CreatedByActorId,
                    0);

            // Threshold alerts (e.g CPU/RAM)
            if (AlertTypeMetadata.IsThreshold(rule.Type))
            {
                state.RegisterMatch();

                if (!state.CanFire(rule))
                {
                    await uow.Alerters.UpdateStateAsync(state, token);
                    await uow.CommitAsync(token);
                    return;
                }
            }
            // event alerts (e.g DeploymentAutoUpdated, etc.)
            else
            {
                // No counter logic at all
                if (!rule.CanTrigger(utcNow, state))
                {
                    await uow.Alerters.UpdateStateAsync(state, token);
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
                match.ResourceType,
                rule.CreatedByActorId);

            state.MarkTriggered(utcNow);

            // Reset only for threshold alerts
            if (AlertTypeMetadata.IsThreshold(rule.Type))
                state.Reset();

            await uow.Alerters.UpdateStateAsync(state, token);
            await uow.CommitAsync(token);

            resultSink.Add(evt);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Alert evaluation DB work failed.");
        }
    }
}