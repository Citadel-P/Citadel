using Application.Services.Identity;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, PermissionLevel.Write, ResourceIdProperty = nameof(AddTeamMember.TeamId))]
public sealed record AddTeamMember(Guid TeamId, Guid MemberActorId) : ICommand<Result<TeamDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<AddTeamMember>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.MemberActorId).NotEmpty();
        }
    }
}

internal sealed class AddTeamMemberHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    ILicenseEntitlementService entitlementService,
    IUserContextAccessor userContext) : ICommandHandler<AddTeamMember, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(AddTeamMember command, CancellationToken cancellationToken)
    {
        await unitOfWork.Actors.AcquireRunAsLockAsync(command.MemberActorId, cancellationToken);
        var state = await unitOfWork.Teams.GetMemberAssignmentStateAsync(command.TeamId, command.MemberActorId, cancellationToken);
        if (state.Team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        if (!state.MemberExists)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided User or Service Account does not exist"));

        if (state.IsArchived)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided Service Account does not exist"));

        if (state.HasMember)
            return Result.Failure<TeamDetails>(new ConflictError("The Actor is already a member of the Team"));

        var requiresCustomAccess = state.IsServiceAccount
            || await unitOfWork.Actors.HasCustomAccessConfigurationAsync(
                [state.Team.ActorId], cancellationToken);
        if (requiresCustomAccess)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.CustomAccessControl,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<TeamDetails>(entitlementError);
        }

        var oldSnapshot = await IdentityActivity.CaptureTeamAsync(unitOfWork, state.Team.Id, cancellationToken);
        if (oldSnapshot is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        await unitOfWork.Teams.AddMemberAsync(command.TeamId, command.MemberActorId, cancellationToken);
        var newSnapshot = IdentityActivity.WithMember(oldSnapshot, command.MemberActorId, add: true);
        await unitOfWork.ActivityEventRepository.AddAsync(
            IdentityActivity.Create(
                state.Team.Id,
                state.Team.Name,
                userContext.Current.ActorId,
                ActivityEventType.TeamUpdated,
                new TeamUpdated(oldSnapshot, newSnapshot)),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await evictor.EvictActorAsync(command.MemberActorId, cancellationToken);

        return state.Team;
    }
}
