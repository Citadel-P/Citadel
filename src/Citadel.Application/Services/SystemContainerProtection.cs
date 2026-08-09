using Domain.Contracts.Interfaces;

namespace Application.Services;

internal static class SystemContainerProtection
{
    internal const string ErrorMessage =
        "Citadel system containers cannot be managed from Citadel. Manage them from the Docker host.";
    internal const string SwarmTaskErrorMessage =
        "Docker Swarm task containers cannot be managed directly. Manage their Service or Stack instead.";

    internal static async Task<string?> GetErrorAsync(
        IUnitOfWork unitOfWork,
        IEnumerable<string> containerIds,
        CancellationToken cancellationToken)
    {
        var ids = containerIds
            .Where(id => !string.IsNullOrWhiteSpace(id))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        if (ids.Length == 0)
            return null;

        var containers = await unitOfWork.Containers.GetByIdsAsync(ids, cancellationToken);
        if (containers.Any(container => container.IsSystem))
            return ErrorMessage;
        return containers.Any(container => container.IsSwarmTask)
            ? SwarmTaskErrorMessage
            : null;
    }
}
