using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, ResourceAction.Update)]
public sealed record AddTeamResourceAccess(Guid TeamId, ResourceType ResourceType, Guid ResourceId, ResourceAction Action) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<AddTeamResourceAccess>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.ResourceId).NotEmpty();
            RuleFor(x => x.ResourceType).IsInEnum();
            RuleFor(x => x.Action).IsInEnum();
            RuleFor(x => x)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.Action))
                .WithMessage(x => $"Invalid permission: [{x.ResourceType}]-[{x.Action}] is not an allowed combination.");
        }
    }
}

internal sealed class AddTeamResourceAccessHandler(IUnitOfWork unitOfWork, IActorResourceAccessService actorResourceAccessService)
    : ICommandHandler<AddTeamResourceAccess, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(AddTeamResourceAccess command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.TeamId, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var result = await actorResourceAccessService.AddResourceAccessAsync(
            team.ActorId,
            command.ResourceType,
            command.ResourceId,
            command.Action,
            cancellationToken);

        if (result.IsFailure(out var error))
            return Result.Failure<TeamDetails>(error);

        return team;
    }
}
