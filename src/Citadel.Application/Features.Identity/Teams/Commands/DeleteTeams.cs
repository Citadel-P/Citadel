using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, PermissionLevel.Execute)]
public sealed record DeleteTeams(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteTeamsHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteTeams, Result>
{
    public async ValueTask<Result> Handle(DeleteTeams command, CancellationToken cancellationToken)
    {
        var teams = await unitOfWork.Teams.GetAllAsync(command.Ids, cancellationToken);
        if (teams is null || !teams.Any())
            return Result.Failure(new NotFoundError("No teams found matching the provided IDs."));

        await unitOfWork.Teams.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
