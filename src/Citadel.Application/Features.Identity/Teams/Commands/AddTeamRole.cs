using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain;
using Domain.Entities.Activities;
using Application.Services.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, PermissionLevel.Write, ResourceIdProperty = nameof(AddTeamRole.TeamId))]
public sealed record AddTeamRole(Guid TeamId, Guid RoleId) : ICommand<Result<TeamDetails>>, IAdministratorRequest
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

internal sealed class AddTeamRoleHandler(
    IUnitOfWork unitOfWork,
    IActorRoleService actorRoleService,
    IUserContextAccessor userContext) : ICommandHandler<AddTeamRole, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(AddTeamRole command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.TeamId, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var oldSnapshot = await IdentityActivity.CaptureTeamAsync(unitOfWork, team.Id, cancellationToken);
        if (oldSnapshot is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));
        var newSnapshot = IdentityActivity.WithRole(oldSnapshot, command.RoleId, add: true);
        await unitOfWork.ActivityEventRepository.AddAsync(
            IdentityActivity.Create(
                team.Id,
                team.Name,
                userContext.Current.ActorId,
                ActivityEventType.TeamUpdated,
                new TeamUpdated(oldSnapshot, newSnapshot)),
            cancellationToken);

        var result = await actorRoleService.AssignRoleAsync(team.ActorId, command.RoleId, cancellationToken);
        if (result.IsFailure(out var error))
            return Result.Failure<TeamDetails>(error);

        return team;
    }
}
