using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Queries;

public sealed record GetUserPreferences : IQuery<Result<UserPreferencesDetails>>;

internal sealed class GetUserPreferencesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : IQueryHandler<GetUserPreferences, Result<UserPreferencesDetails>>
{
    public async ValueTask<Result<UserPreferencesDetails>> Handle(GetUserPreferences query, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        if (userId == Guid.Empty)
            return Result.Failure<UserPreferencesDetails>(new UnauthorizedError("Missing user context"));

        var preferences = await unitOfWork.UserPreferences.GetAsync(userId, cancellationToken);
        return preferences is null
            ? Result.Success(new UserPreferencesDetails(null, UserDateTimeFormat.System, UserTheme.System, false))
            : Result.Success(new UserPreferencesDetails(
                preferences.TimeZone,
                preferences.DateTimeFormat,
                preferences.Theme,
                true));
    }
}
