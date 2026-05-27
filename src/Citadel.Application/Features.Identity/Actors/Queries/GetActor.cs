using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Identity.Actors.Queries;

public sealed record GetActor(Guid Id) : IQuery<Result<Actor>>
{
    internal sealed class Validator : AbstractValidator<GetActor>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
        }
    }
}

internal sealed class GetActorHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetActor, Result<Actor>>
{
    public async ValueTask<Result<Actor>> Handle(GetActor query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        if (user is null || user.IsAuthenticated != true)
        {
            return Result.Failure<Actor>(new UnauthorizedError("Missing user context"));
        }

        if (!user.IsAdmin)
        {
            return Result.Failure<Actor>(new ForbiddenError("Only administrators can view actors."));
        }

        var actor = await unitOfWork.Actors.GetById(query.Id, cancellationToken);
        return actor is null
            ? Result.Failure<Actor>(new NotFoundError("The provided actor does not exist"))
            : Result.Success(actor);
    }
}
