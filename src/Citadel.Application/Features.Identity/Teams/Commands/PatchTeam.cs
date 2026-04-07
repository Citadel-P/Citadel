using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, ResourceAction.Update)]
public sealed record PatchTeam(Guid Id, JsonMergePatchDocument<PatchTeamModel> Patch) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : PatchCommandValidator<PatchTeam, PatchTeamModel>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: RoleJsonContext.Default.PatchTeamModel,
                modelValidator: new PatchTeamModelValidator())
        {
        }
    }

    internal sealed class PatchTeamModelValidator : AbstractValidator<PatchTeamModel>;
}

internal sealed class PatchTeamHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchTeam, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(PatchTeam command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.Id, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var actor = await unitOfWork.Actors.GetById(team.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided actor does not exist"));

        var current = new PatchTeamModel(team.IsEnabled);
        var patched = command.Patch.ApplyTo(current, RoleJsonContext.Default.PatchTeamModel);

        if (patched.IsEnabled.HasValue && patched.IsEnabled.Value != actor.IsEnabled)
        {
            var setEnabledResult = actor.SetEnabled(patched.IsEnabled.Value);
            if (setEnabledResult.IsFailure(out var error))
                return Result.Failure<TeamDetails>(error);
        }

        await unitOfWork.Actors.UpdateAsync(actor, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new TeamDetails(team.Id, team.Name, team.ActorId, actor.IsEnabled);
    }
}
