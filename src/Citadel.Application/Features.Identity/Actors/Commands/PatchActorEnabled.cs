using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Identity.Actors.Commands;

public sealed record PatchActorEnabled(Guid Id, bool IsEnabled) : ICommand<Result<Actor>>
{
    internal sealed class Validator : AbstractValidator<PatchActorEnabled>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
        }
    }
}

internal sealed class PatchActorEnabledHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor)
    : ICommandHandler<PatchActorEnabled, Result<Actor>>
{
    public async ValueTask<Result<Actor>> Handle(PatchActorEnabled command, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        if (user is null || user.Identity?.IsAuthenticated != true)
        {
            return Result.Failure<Actor>(new UnauthorizedError("Missing user context"));
        }

        if (!user.IsAdmin())
        {
            return Result.Failure<Actor>(new ForbiddenError("Only administrators can update actors."));
        }

        var actor = await unitOfWork.Actors.GetById(command.Id, cancellationToken);
        if (actor is null)
        {
            return Result.Failure<Actor>(new NotFoundError("The provided actor does not exist"));
        }

        var result = actor.SetEnabled(command.IsEnabled);
        if (result.IsFailure(out var error))
        {
            return Result.Failure<Actor>(error);
        }

        await unitOfWork.Actors.UpdateAsync(actor, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return actor;
    }
}
