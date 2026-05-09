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

[RequirePermission(ResourceType.Team, PermissionLevel.Write)]
public sealed record RemoveTeamResourceAccess(
    Guid TeamId,
    ResourceType ResourceType,
    Guid ResourceId,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<RemoveTeamResourceAccess>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.ResourceId).NotEmpty();
            RuleFor(x => x.ResourceType).IsInEnum();
            RuleFor(x => x.PermissionLevel).IsInEnum();
            RuleForEach(x => x.SpecificPermissions).IsInEnum();
            RuleFor(x => x)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.PermissionLevel, x.SpecificPermissions))
                .WithMessage(x => $"Invalid permission: [{x.ResourceType}]-[{x.PermissionLevel}] with specifics [{string.Join(", ", x.SpecificPermissions ?? [])}] is not an allowed combination.");
        }
    }
}

internal sealed class RemoveTeamResourceAccessHandler(IUnitOfWork unitOfWork, IActorResourceAccessService actorResourceAccessService)
    : ICommandHandler<RemoveTeamResourceAccess, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(RemoveTeamResourceAccess command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.TeamId, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var result = await actorResourceAccessService.RemoveResourceAccessAsync(
            team.ActorId,
            command.ResourceType,
            command.ResourceId,
            command.PermissionLevel,
            command.SpecificPermissions,
            cancellationToken);

        if (result.IsFailure(out var error))
            return Result.Failure<TeamDetails>(error);

        var persistedResourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(team.ActorId, cancellationToken);
        return team with
        {
            ResourceAccesses = persistedResourceAccesses.Select(x => new ResourceAccessView(x.ResourceType, x.ResourceId, x.ResourceName, x.PermissionLevel, x.SpecificPermissions))
        };
    }
}
