using Application.Services.Alerts;
using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

public sealed record DeleteAlertRules(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteAlertRulesHandler(IUnitOfWork unitOfWork, AlertRuleCache alertRuleCache) : ICommandHandler<DeleteAlertRules, Result>
{
    public async ValueTask<Result> Handle(DeleteAlertRules command, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.AlertRules.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        // reload cache
        await alertRuleCache.ReloadAsync(cancellationToken);

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No alert rules found matching the provided IDs for deletion."));
    }
}
