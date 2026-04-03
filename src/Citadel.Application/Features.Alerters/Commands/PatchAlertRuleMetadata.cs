using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

public sealed record PatchAlertRuleMetadata(Guid Id, JsonMergePatchDocument<AlertRule> Patch) : ICommand<Result<AlertRule>>
{
    internal sealed class Validator : AbstractValidator<PatchAlertRuleMetadata>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Patch).NotNull();
        }
    }
}

internal sealed class PatchAlertRuleMetadataHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache) : ICommandHandler<PatchAlertRuleMetadata, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(PatchAlertRuleMetadata command, CancellationToken cancellationToken)
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

        rule.PartialUpdate(description: patchedRule.Description);

        await unitOfWork.AlertRules.UpdateAsync(rule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        alertRuleCache.Upsert(rule);
        return rule;
    }
}
