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
public sealed record AddTeamMember(Guid TeamId, Guid UserId) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<AddTeamMember>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.UserId).NotEmpty();
        }
    }
}

internal sealed class AddTeamMemberHandler(IUnitOfWork unitOfWork, IActorScopeEvictor evictor) : ICommandHandler<AddTeamMember, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(AddTeamMember command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Teams.GetMemberAssignmentStateAsync(command.TeamId, command.UserId, cancellationToken);
        if (state.Team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        if (!state.UserExists)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided user does not exist"));

        if (state.HasMember)
            return Result.Failure<TeamDetails>(new ConflictError("The user is already a member of the team"));

        await unitOfWork.Teams.AddMemberAsync(command.TeamId, command.UserId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await evictor.EvictUsers(new[] { command.UserId }, cancellationToken);

        return state.Team;
    }
}
