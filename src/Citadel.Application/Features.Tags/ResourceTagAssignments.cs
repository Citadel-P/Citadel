using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Features.Tags;

internal static class ResourceTagAssignments
{
    internal static async Task<Result<IReadOnlyList<TagSummary>>> ReplaceAsync(
        IUnitOfWork unitOfWork,
        TaggableResourceType resourceType,
        Guid resourceId,
        IReadOnlyCollection<Guid>? tagIds,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var normalizedTagIds = NormalizeTagIds(tagIds);
        if (!await unitOfWork.ResourceTags.AllTagsExistAsync(normalizedTagIds, cancellationToken))
        {
            return Result.Failure<IReadOnlyList<TagSummary>>(new BadRequestError("One or more tags do not exist."));
        }

        await unitOfWork.ResourceTags.ReplaceForResourceAsync(
            resourceType,
            resourceId,
            normalizedTagIds,
            actorId,
            DateTime.UtcNow,
            cancellationToken);

        return Result.Success(await unitOfWork.ResourceTags.GetForResourceAsync(resourceType, resourceId, cancellationToken));
    }

    internal static async Task AssignAsync(
        IUnitOfWork unitOfWork,
        TaggableResourceType resourceType,
        Guid resourceId,
        Action<IReadOnlyList<TagSummary>> assign,
        CancellationToken cancellationToken)
    {
        assign(await unitOfWork.ResourceTags.GetForResourceAsync(resourceType, resourceId, cancellationToken));
    }

    internal static async Task AssignBatchAsync<TResource>(
        IUnitOfWork unitOfWork,
        TaggableResourceType resourceType,
        IReadOnlyList<TResource> resources,
        Func<TResource, Guid> getId,
        Action<TResource, IReadOnlyList<TagSummary>> assign,
        CancellationToken cancellationToken)
    {
        if (resources.Count == 0)
            return;

        var tagsByResourceId = await unitOfWork.ResourceTags.GetForResourcesAsync(
            resourceType,
            [.. resources.Select(getId)],
            cancellationToken);

        foreach (var resource in resources)
        {
            assign(
                resource,
                tagsByResourceId.TryGetValue(getId(resource), out var tags) ? tags : []);
        }
    }

    internal static IReadOnlyList<TResource> FilterByTags<TResource>(
        IEnumerable<TResource> resources,
        IReadOnlyCollection<Guid>? tagIds,
        Func<TResource, IReadOnlyList<TagSummary>> getTags)
    {
        var normalizedTagIds = NormalizeTagIds(tagIds);
        if (normalizedTagIds.Length == 0)
            return [.. resources];

        var requiredTagIds = normalizedTagIds.ToHashSet();
        return [.. resources.Where(resource => getTags(resource).Any(tag => requiredTagIds.Contains(tag.Id)))];
    }

    private static Guid[] NormalizeTagIds(IReadOnlyCollection<Guid>? tagIds)
        => tagIds is null ? [] : [.. tagIds.Where(id => id != Guid.Empty).Distinct()];
}
