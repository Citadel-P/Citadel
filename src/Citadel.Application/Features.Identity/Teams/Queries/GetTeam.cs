using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Queries;

[RequirePermission(ResourceType.Team, PermissionLevel.Read)]
public sealed record GetTeam(Guid Id) : IQuery<Result<TeamDetails>>;

internal sealed class GetTeamHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetTeam, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(GetTeam query, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(query.Id, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError($"Team with ID {query.Id} does not exist"));

        var resourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(team.ActorId, cancellationToken);

        return Result.Success(team with
        {
            ResourceAccesses = resourceAccesses.Select(x => new ResourceAccessView(x.ResourceType, x.ResourceId, x.ResourceName, x.PermissionLevel, x.SpecificPermissions))
        });
    }
}
