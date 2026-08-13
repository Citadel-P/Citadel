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

[RequirePermission(ResourceType.Team, PermissionLevel.Write)]
public sealed record RenameTeam(Guid Id, string Name) : ICommand<Result<TeamDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<RenameTeam>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenameTeamHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<RenameTeam, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(RenameTeam command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Teams.GetTeamUpdateStateAsync(command.Id, command.Name, cancellationToken);
        if (state.Team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        if (state.NameExists)
            return Result.Failure<TeamDetails>(new ConflictError("Name already exists"));

        var oldName = state.Team.Name;
        state.Team.Rename(command.Name);
        await unitOfWork.Teams.UpdateAsync(state.Team, cancellationToken);

        if (!string.Equals(oldName, state.Team.Name, StringComparison.Ordinal))
        {
            await unitOfWork.ActivityEventRepository.AddAsync(
                IdentityActivity.Create(
                    state.Team.Id,
                    state.Team.Name,
                    userContext.Current.ActorId,
                    ActivityEventType.TeamRenamed,
                    new TeamRenamed(oldName, state.Team.Name)),
                cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        return new TeamDetails(state.Team.Id, state.Team.Name, state.Team.ActorId, state.IsEnabled);
    }
}
