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
public sealed record RemoveTeamMember(Guid TeamId, Guid UserId) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<RemoveTeamMember>
    {
        public Validator()
        {
            RuleFor(x => x.TeamId).NotEmpty();
            RuleFor(x => x.UserId).NotEmpty();
        }
    }
}

internal sealed class RemoveTeamMemberHandler(IUnitOfWork unitOfWork) : ICommandHandler<RemoveTeamMember, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(RemoveTeamMember command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Teams.GetMemberAssignmentStateAsync(command.TeamId, command.UserId, cancellationToken);
        if (state.Team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        if (!state.UserExists)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided user does not exist"));

        if (!state.HasMember)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided user is not a member of the team"));

        await unitOfWork.Teams.RemoveMemberAsync(command.TeamId, command.UserId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return state.Team;
    }
}
