namespace Domain.Contracts.Resources.Networks;

internal static class DockerNetworkSystemClassifier
{
    internal static bool IsSystem(string? name, bool ingress)
        => ingress
           || string.Equals(name, "bridge", StringComparison.OrdinalIgnoreCase)
           || string.Equals(name, "host", StringComparison.OrdinalIgnoreCase)
           || string.Equals(name, "none", StringComparison.OrdinalIgnoreCase)
           || string.Equals(name, "nat", StringComparison.OrdinalIgnoreCase)
           || string.Equals(name, "docker_gwbridge", StringComparison.OrdinalIgnoreCase);
}
