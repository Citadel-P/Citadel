using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Releases, ResourceIdProperty = nameof(GetStackReleases.StackId))]
public sealed record GetStackReleases(Guid StackId) : IQuery<Result<IEnumerable<StackRelease>>>;

internal sealed class GetStackReleasesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetStackReleases, Result<IEnumerable<StackRelease>>>
{
    public async ValueTask<Result<IEnumerable<StackRelease>>> Handle(GetStackReleases query, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(query.StackId, cancellationToken);
        if (stack == null)
        {
            return Result.Failure<IEnumerable<StackRelease>>(new NotFoundError($"Stack with ID {query.StackId} does not exist"));
        }

        var releases = await unitOfWork.Stacks.GetReleasesByStackIdAsync(query.StackId, cancellationToken);
        var currentVersion = stack.CurrentStackRelease?.Version;
        return Result.Success(releases.Where(release =>
            release.Id != stack.CurrentStackReleaseId &&
            release.Version != currentVersion &&
            release.IsRollbackCandidate()));
    }
}
