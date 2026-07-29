using Domain.Contracts.Interfaces;

namespace Application.Services;

internal static class SystemContainerProtection
{
    internal const string ErrorMessage =
        "Citadel system containers cannot be managed from Citadel. Manage them from the Docker host.";

    internal static async Task<bool> ContainsSystemContainerAsync(
        IUnitOfWork unitOfWork,
        IEnumerable<string> containerIds,
        CancellationToken cancellationToken)
    {
        var ids = containerIds
            .Where(id => !string.IsNullOrWhiteSpace(id))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        if (ids.Length == 0)
            return false;

        var containers = await unitOfWork.Containers.GetByIdsAsync(ids, cancellationToken);
        return containers.Any(container => container.IsSystem);
    }
}
