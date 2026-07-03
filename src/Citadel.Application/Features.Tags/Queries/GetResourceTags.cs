using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Tags.Queries;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read)]
public sealed record GetDeploymentTags(Guid Id) : IQuery<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read)]
public sealed record GetStackTags(Guid Id) : IQuery<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetPlatformTags(Guid Id) : IQuery<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read)]
public sealed record GetGitRepositoryTags(Guid Id) : IQuery<Result<IReadOnlyList<TagSummary>>>;

internal sealed class GetDeploymentTagsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetDeploymentTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(GetDeploymentTags query, CancellationToken cancellationToken)
        => await ResourceTagResourceAccess.GetAsync(
            unitOfWork,
            TaggableResourceType.Deployment,
            query.Id,
            async () => await unitOfWork.Deployments.GetAsync(query.Id, cancellationToken) is not null,
            "Deployment",
            cancellationToken);
}

internal sealed class GetStackTagsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetStackTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(GetStackTags query, CancellationToken cancellationToken)
        => await ResourceTagResourceAccess.GetAsync(
            unitOfWork,
            TaggableResourceType.Stack,
            query.Id,
            async () => await unitOfWork.Stacks.ExistsAsync(query.Id, cancellationToken),
            "Stack",
            cancellationToken);
}

internal sealed class GetPlatformTagsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetPlatformTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(GetPlatformTags query, CancellationToken cancellationToken)
        => await ResourceTagResourceAccess.GetAsync(
            unitOfWork,
            TaggableResourceType.Platform,
            query.Id,
            async () => await unitOfWork.Platforms.ExistsAsync(query.Id, cancellationToken),
            "Platform",
            cancellationToken);
}

internal sealed class GetGitRepositoryTagsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetGitRepositoryTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(GetGitRepositoryTags query, CancellationToken cancellationToken)
        => await ResourceTagResourceAccess.GetAsync(
            unitOfWork,
            TaggableResourceType.GitRepository,
            query.Id,
            async () => await unitOfWork.GitRepositories.GetAsync(query.Id, cancellationToken) is not null,
            "Git repository",
            cancellationToken);
}

internal static class ResourceTagResourceAccess
{
    internal static async Task<Result<IReadOnlyList<TagSummary>>> GetAsync(
        IUnitOfWork unitOfWork,
        TaggableResourceType resourceType,
        Guid resourceId,
        Func<Task<bool>> resourceExists,
        string resourceLabel,
        CancellationToken cancellationToken)
    {
        if (!await resourceExists())
            return Result.Failure<IReadOnlyList<TagSummary>>(new NotFoundError($"{resourceLabel} with ID {resourceId} does not exist"));

        return Result.Success(await unitOfWork.ResourceTags.GetForResourceAsync(resourceType, resourceId, cancellationToken));
    }
}
