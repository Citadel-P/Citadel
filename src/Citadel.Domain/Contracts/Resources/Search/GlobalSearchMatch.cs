using Hosting.Common;

namespace Domain.Contracts.Resources.Search;

public sealed record GlobalSearchMatch(
    Guid Id,
    ResourceType ResourceType,
    string Name,
    string? SecondaryText,
    string? Status,
    Guid? ParentId,
    ResourceType? ParentResourceType,
    string? ParentName,
    int MatchRank);

public interface IGlobalSearchRepository
{
    Task<IReadOnlyList<GlobalSearchMatch>> SearchAsync(
        Guid userId,
        bool isAdmin,
        IReadOnlyCollection<ResourceType> resourceTypes,
        string query,
        int limitPerType,
        int totalLimit,
        CancellationToken cancellationToken);
}
