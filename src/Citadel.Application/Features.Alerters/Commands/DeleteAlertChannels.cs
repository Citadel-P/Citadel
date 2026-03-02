using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

public sealed record DeleteAlertChannels(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteAlertChannelsHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteAlertChannels, Result>
{
    public async ValueTask<Result> Handle(DeleteAlertChannels command, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.AlertRules.RemoveChannelsRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        
        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No alert channels found matching the provided IDs for deletion."));
    }
}
