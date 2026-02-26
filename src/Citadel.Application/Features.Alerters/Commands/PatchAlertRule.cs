using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

public sealed record PatchAlertRule(Guid Id, JsonMergePatchDocument<AlertRule> Patch) : ICommand<Result<AlertRule>>
{
    internal sealed class Validator : AbstractValidator<PatchAlertRule>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Patch).NotNull();
        }
    }
}

internal sealed class PatchAlertRuleHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(PatchAlertRule command, CancellationToken cancellationToken)
    {
        var rule = await unitOfWork.AlertRules.GetByIdAsync(command.Id, cancellationToken);
        if (rule is null)
        {
            return Result.Failure<AlertRule>(new NotFoundError("The provided alert rule does not exist"));
        }

        AlertRule patchedRule;
        try
        {
            patchedRule = command.Patch.ApplyTo(rule, AlertRuleJsonContext.Default.AlertRule);
        }
        catch (Exception ex) when (ex is ArgumentException or InvalidOperationException)
        {
            return Result.Failure<AlertRule>(new BadRequestError(ex.Message));
        }

        rule.PartialUpdate(
            type: patchedRule.Type,
            severity: patchedRule.Severity,
            cooldownSeconds: patchedRule.CooldownSeconds,
            isEnabled: patchedRule.IsEnabled,
            scope: patchedRule.Scope,
            requiredMatches: patchedRule.RequiredMatches,
            threshold: patchedRule.Threshold,
            limitedTo: patchedRule.LimitedTo,
            quietHours: patchedRule.QuietHours);

        await unitOfWork.AlertRules.UpdateAsync(rule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return rule;
    }
}
