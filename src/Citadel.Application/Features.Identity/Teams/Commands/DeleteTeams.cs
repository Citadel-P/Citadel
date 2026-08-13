using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, PermissionLevel.Execute)]
public sealed record DeleteTeams(IEnumerable<Guid> Ids) : ICommand<Result>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<DeleteTeams>
    {
        public Validator()
            => RuleFor(command => command.Ids).NotNull().NotEmpty();
    }
}

internal sealed class DeleteTeamsHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    IAdministratorGuard administratorGuard,
    IUserContextAccessor userContext) : ICommandHandler<DeleteTeams, Result>
{
    public async ValueTask<Result> Handle(DeleteTeams command, CancellationToken cancellationToken)
    {
        var teams = await unitOfWork.Teams.GetAllAsync(command.Ids, cancellationToken);
        if (teams is null || !teams.Any())
            return Result.Failure(new NotFoundError("No teams found matching the provided IDs."));

        var teamArray = teams.ToArray();
        var snapshots = new List<(Domain.Entities.Identity.Team Team, TeamActivitySnapshot Snapshot)>(teamArray.Length);
        var affectedActorIds = new List<Guid>();
        foreach (var team in teamArray)
        {
            var snapshot = await IdentityActivity.CaptureTeamAsync(unitOfWork, team.Id, cancellationToken);
            if (snapshot is not null)
                snapshots.Add((team, snapshot));
            affectedActorIds.AddRange(await unitOfWork.Teams.GetAffectedPrincipalActorIdsAsync(team.ActorId, cancellationToken));
        }

        await unitOfWork.Teams.RemoveRangeAsync(teamArray.Select(static team => team.Id), cancellationToken);

        var guardResult = await administratorGuard.EnsureAdministratorRemainsAsync(cancellationToken);
        if (guardResult.IsFailure(out var guardError))
            return Result.Failure(guardError);

        foreach (var (team, snapshot) in snapshots)
        {
            await unitOfWork.ActivityEventRepository.AddAsync(
                IdentityActivity.Create(
                    team.Id,
                    team.Name,
                    userContext.Current.ActorId,
                    ActivityEventType.TeamDeleted,
                    new TeamDeleted(snapshot)),
                cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);
        await evictor.EvictActors(affectedActorIds, cancellationToken);
        return Result.Success();
    }
}
