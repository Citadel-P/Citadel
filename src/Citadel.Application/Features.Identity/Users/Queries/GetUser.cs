using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Queries;

[RequirePermission(ResourceType.User, PermissionLevel.Read)]
public sealed record GetUser(Guid Id) : IQuery<Result<UserDetails>>;

internal sealed class GetUserHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(GetUser query, CancellationToken cancellationToken)
    {
        var user = await unitOfWork.Users.GetDetailsAsync(query.Id, cancellationToken);
        if (user is null)
            return Result.Failure<UserDetails>(new NotFoundError($"User with ID {query.Id} does not exist"));

        var resourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(user.ActorId, cancellationToken);

        return Result.Success(new UserDetails(
            user.Id,
            user.Name,
            user.Email,
            user.ActorId,
            user.IsEnabled,
            user.CreatedAt,
            user.CreatedByActorId,
            user.Teams,
            user.Roles,
            ResourceAccesses: resourceAccesses.Select(x => new ResourceAccessView(x.ResourceType, x.ResourceId, x.PermissionLevel, x.SpecificPermissions))));
    }
}
