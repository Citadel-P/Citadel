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
public sealed record AddTeamRole(Guid TeamId, Guid RoleId) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<AddTeamRole>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.RoleId).NotEmpty();
        }
    }
}

internal sealed class AddTeamRoleHandler(IUnitOfWork unitOfWork, IActorRoleService actorRoleService) : ICommandHandler<AddTeamRole, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(AddTeamRole command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.TeamId, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var result = await actorRoleService.AssignRoleAsync(team.ActorId, command.RoleId, cancellationToken);
        if (result.IsFailure(out var error))
            return Result.Failure<TeamDetails>(error);

        return team;
    }
}
