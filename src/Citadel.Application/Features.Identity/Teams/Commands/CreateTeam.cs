using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, ResourceAction.Create)]
public sealed record CreateTeam(string Name) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<CreateTeam>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class CreateTeamHandler(IUnitOfWork unitOfWork) : ICommandHandler<CreateTeam, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(CreateTeam command, CancellationToken cancellationToken)
    {
        var conflicts = await unitOfWork.Teams.GetConflictsAsync(command.Name, null, cancellationToken);
        if (conflicts)
            return Result.Failure<TeamDetails>(new ConflictError("Name already exists"));

        var actor = Actor.Create(ActorType.Team, new ActorMetadata(command.Name));
        var team = Team.Create(command.Name, actor.Id);

        await unitOfWork.Actors.AddAsync(actor, cancellationToken);
        await unitOfWork.Teams.AddAsync(team, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new TeamDetails(team.Id, team.Name, team.ActorId, actor.IsEnabled);
    }
}
