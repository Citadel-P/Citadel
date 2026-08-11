using Domain.Entities.Platforms;

namespace Application.Services;

internal static class SwarmNodeAgentInfrastructure
{
    internal const string SystemLabel = "com.citadel.system";
    internal const string SystemRoleLabel = "com.citadel.system-role";
    internal const string PlatformIdLabel = "com.citadel.platform-id";
    internal const string ClusterIdLabel = "com.citadel.swarm-cluster-id";
    internal const string SystemRole = "swarm-node-agent";

    internal static IReadOnlyDictionary<string, string> OwnershipLabels(Platform platform)
        => new Dictionary<string, string>(StringComparer.Ordinal)
        {
            [SystemLabel] = "true",
            [SystemRoleLabel] = SystemRole,
            [PlatformIdLabel] = platform.Id.ToString("D"),
            [ClusterIdLabel] = platform.ClusterId!
        };

    internal static bool HasOwnership(SwarmServiceProjection service, Platform platform)
        => HasOwnership(service.Labels, platform.Id, platform.ClusterId);

    internal static bool HasOwnership(
        IReadOnlyDictionary<string, string> labels,
        Platform platform)
        => HasOwnership(labels, platform.Id, platform.ClusterId);

    internal static string? GetServiceDriftReason(
        Platform platform,
        SwarmNodeAgentInstallation installation,
        IReadOnlyList<SwarmServiceProjection> services)
    {
        var service = string.IsNullOrWhiteSpace(installation.DockerServiceId)
            ? null
            : services.FirstOrDefault(item => string.Equals(
                item.DockerServiceId,
                installation.DockerServiceId,
                StringComparison.Ordinal));
        if (service is null)
        {
            var replacement = services.FirstOrDefault(item => string.Equals(
                item.Name,
                installation.DockerServiceName,
                StringComparison.Ordinal));
            return replacement is null
                ? "The Citadel node-agent Service is missing. Run Repair coverage."
                : "The Citadel node-agent Service identity changed outside Citadel. Run Repair coverage.";
        }

        if (!HasOwnership(service, platform))
            return "The Citadel node-agent Service ownership labels changed outside Citadel. Run Repair coverage.";
        if (!service.Mode.Equals("Global", StringComparison.OrdinalIgnoreCase))
            return "The Citadel node-agent Service is no longer global. Run Repair coverage.";
        if (!string.IsNullOrWhiteSpace(installation.AgentImageDigest)
            && !string.Equals(
                ExtractDigest(service.Image),
                installation.AgentImageDigest,
                StringComparison.OrdinalIgnoreCase))
        {
            return "The Citadel node-agent Service image changed outside Citadel. Run Repair coverage.";
        }

        return null;
    }

    private static bool HasOwnership(
        IReadOnlyDictionary<string, string> labels,
        Guid platformId,
        string? clusterId)
        => labels.TryGetValue(SystemLabel, out var system)
           && string.Equals(system, "true", StringComparison.OrdinalIgnoreCase)
           && labels.TryGetValue(SystemRoleLabel, out var role)
           && string.Equals(role, SystemRole, StringComparison.Ordinal)
           && labels.TryGetValue(PlatformIdLabel, out var labelPlatformId)
           && string.Equals(labelPlatformId, platformId.ToString("D"), StringComparison.OrdinalIgnoreCase)
           && labels.TryGetValue(ClusterIdLabel, out var labelClusterId)
           && string.Equals(labelClusterId, clusterId, StringComparison.Ordinal);

    private static string? ExtractDigest(string image)
    {
        var separator = image.LastIndexOf('@');
        return separator >= 0 && separator < image.Length - 1
            ? image[(separator + 1)..]
            : null;
    }
}
