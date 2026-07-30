using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services;

internal static class RunAsActorAuthorization
{
    public static Result EnsureAllowed(IUserContext user, Guid runAsActorId)
        => user.IsAdmin || runAsActorId == user.ActorId
            ? Result.Success()
            : Result.Failure(new ForbiddenError("Only administrators can run operations as another user."));
}
