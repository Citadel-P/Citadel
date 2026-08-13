using Application.Services.Identity;
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

[RequirePermission(ResourceType.Team, PermissionLevel.Write, ResourceIdProperty = nameof(RemoveTeamMember.TeamId))]
public sealed record RemoveTeamMember(Guid TeamId, Guid MemberActorId) : ICommand<Result<TeamDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<RemoveTeamMember>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.MemberActorId).NotEmpty();
        }
    }
}

internal sealed class RemoveTeamMemberHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    IAdministratorGuard administratorGuard,
    IUserContextAccessor userContext) : ICommandHandler<RemoveTeamMember, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(RemoveTeamMember command, CancellationToken cancellationToken)
    {
        await unitOfWork.Actors.AcquireRunAsLockAsync(command.MemberActorId, cancellationToken);
        var state = await unitOfWork.Teams.GetMemberAssignmentStateAsync(command.TeamId, command.MemberActorId, cancellationToken);
        if (state.Team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        if (!state.MemberExists)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided User or Service Account does not exist"));

        if (!state.HasMember)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided Actor is not a member of the Team"));

        var oldSnapshot = await IdentityActivity.CaptureTeamAsync(unitOfWork, state.Team.Id, cancellationToken);
        if (oldSnapshot is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        await unitOfWork.Teams.RemoveMemberAsync(command.TeamId, command.MemberActorId, cancellationToken);

        var guardResult = await administratorGuard.EnsureAdministratorRemainsAsync(cancellationToken);
        if (guardResult.IsFailure(out var guardError))
            return Result.Failure<TeamDetails>(guardError);

        var newSnapshot = IdentityActivity.WithMember(oldSnapshot, command.MemberActorId, add: false);
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
