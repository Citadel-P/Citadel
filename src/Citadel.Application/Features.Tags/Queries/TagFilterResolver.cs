using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;

namespace Application.Features.Tags.Queries;

internal sealed record ResolvedTagFilter(IReadOnlyCollection<Guid>? TagIds, bool NoMatch);

internal static class TagFilterResolver
{
    internal static async Task<ResolvedTagFilter> ResolveAsync(
        IUnitOfWork unitOfWork,
        IReadOnlyCollection<string>? tagNames,
        CancellationToken cancellationToken)
    {
        var ids = new List<Guid>();

        var normalizedNames = tagNames?
            .Where(name => !string.IsNullOrWhiteSpace(name))
            .Select(TagValidation.NormalizeName)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray() ?? [];

        foreach (var normalizedName in normalizedNames)
        {
            var tag = await unitOfWork.Tags.GetByNormalizedNameAsync(normalizedName, cancellationToken);
            if (tag is null)
                return new ResolvedTagFilter(ids, NoMatch: true);

            ids.Add(tag.Id);
        }

        return new ResolvedTagFilter(ids.Count == 0 ? null : ids.Distinct().ToArray(), NoMatch: false);
    }
}
