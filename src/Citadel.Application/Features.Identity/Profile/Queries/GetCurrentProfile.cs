using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Queries;

public sealed record GetCurrentProfile : IQuery<Result<CurrentProfileDetails>>;

internal sealed class GetCurrentProfileHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : IQueryHandler<GetCurrentProfile, Result<CurrentProfileDetails>>
{
    public async ValueTask<Result<CurrentProfileDetails>> Handle(GetCurrentProfile query, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        if (userId == Guid.Empty)
            return Result.Failure<CurrentProfileDetails>(new UnauthorizedError("Missing user context"));

        var profile = await unitOfWork.Users.GetCurrentProfileAsync(userId, cancellationToken);
        return profile is null
            ? Result.Failure<CurrentProfileDetails>(new NotFoundError("Current user does not exist"))
            : Result.Success(profile);
    }
}
