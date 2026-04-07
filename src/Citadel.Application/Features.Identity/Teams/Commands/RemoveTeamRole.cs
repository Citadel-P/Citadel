using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Application.Services.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, ResourceAction.Update)]
public sealed record RemoveTeamRole(Guid TeamId, Guid RoleId) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<RemoveTeamRole>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.RoleId).NotEmpty();
        }
    }
}

internal sealed class RemoveTeamRoleHandler(IUnitOfWork unitOfWork, IActorRoleService actorRoleService) : ICommandHandler<RemoveTeamRole, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(RemoveTeamRole command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.TeamId, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var result = await actorRoleService.RemoveRoleAsync(team.ActorId, command.RoleId, cancellationToken);
        if (result.IsFailure(out var error))
            return Result.Failure<TeamDetails>(error);

        return team;
    }
}
