using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Queries;

[RequirePermission(ResourceType.User, ResourceAction.View)]
public sealed record GetUser(Guid Id) : IQuery<Result<UserDetails>>;

internal sealed class GetUserHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(GetUser query, CancellationToken cancellationToken)
    {
        var user = await unitOfWork.Users.GetAsync(query.Id, cancellationToken);
        if (user is null)
            return Result.Failure<UserDetails>(new NotFoundError($"User with ID {query.Id} does not exist"));

        var actor = await unitOfWork.Actors.GetById(user.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided actor does not exist"));

        return Result.Success(new UserDetails(user.Id, user.Name, user.Email, user.ActorId, actor.IsEnabled, user.CreatedAt, user.CreatedByActorId));
    }
}
