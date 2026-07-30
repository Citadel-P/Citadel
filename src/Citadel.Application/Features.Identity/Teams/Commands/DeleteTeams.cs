using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, PermissionLevel.Execute)]
public sealed record DeleteTeams(IEnumerable<Guid> Ids) : ICommand<Result>, IAdministratorRequest;

internal sealed class DeleteTeamsHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    IAdministratorGuard administratorGuard) : ICommandHandler<DeleteTeams, Result>
{
    public async ValueTask<Result> Handle(DeleteTeams command, CancellationToken cancellationToken)
    {
        var teams = await unitOfWork.Teams.GetAllAsync(command.Ids, cancellationToken);
        if (teams is null || !teams.Any())
            return Result.Failure(new NotFoundError("No teams found matching the provided IDs."));

        var teamArray = teams.ToArray();
        var affectedUserIds = new List<Guid>();
        foreach (var team in teamArray)
            affectedUserIds.AddRange(await unitOfWork.Teams.GetUserIdsByActorIdAsync(team.ActorId, cancellationToken));

        await unitOfWork.Teams.RemoveRangeAsync(teamArray.Select(static team => team.Id), cancellationToken);

        var guardResult = await administratorGuard.EnsureAdministratorRemainsAsync(cancellationToken);
        if (guardResult.IsFailure(out var guardError))
            return Result.Failure(guardError);

        await unitOfWork.CommitAsync(cancellationToken);
        await evictor.EvictUsers(affectedUserIds, cancellationToken);
        return Result.Success();
    }
}
