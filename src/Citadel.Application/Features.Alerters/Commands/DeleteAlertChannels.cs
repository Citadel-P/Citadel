using Application.Services.Alerts;
using Hosting.Common;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.AlertChannel, PermissionLevel.Execute)]
public sealed record DeleteAlertChannels(IEnumerable<Guid> Ids) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeleteAlertChannels>
    {
        public Validator()
            => RuleFor(command => command.Ids).NotNull().NotEmpty();
    }
}

internal sealed class DeleteAlertChannelsHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache) : ICommandHandler<DeleteAlertChannels, Result>
{
    public async ValueTask<Result> Handle(DeleteAlertChannels command, CancellationToken cancellationToken)
    {
        var ids = command.Ids.Distinct().ToArray();
        var result = await unitOfWork.AlertRules.RemoveChannelsRangeAsync(ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        if (result > 0)
        {
            alertRuleCache.RemoveChannels(ids);
        }

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No alert channels found matching the provided IDs for deletion."));
    }
}
