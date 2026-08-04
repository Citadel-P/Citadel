namespace Domain.Contracts.Resources.Swarm;

public static class SwarmInventoryLimits
{
    public const int MaximumItems = 500;

    public static int Normalize(int requested) => Math.Clamp(requested, 1, MaximumItems);
}

public sealed record ListSwarmServicesCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmServiceCommand(string PlatformAddress, string ServiceId);
public sealed record ListSwarmTasksCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmTaskCommand(string PlatformAddress, string TaskId);
public sealed record ListSwarmNetworksCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmNetworkCommand(string PlatformAddress, string NetworkId);
public sealed record ListSwarmSecretsCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmSecretCommand(string PlatformAddress, string SecretId);
public sealed record ListSwarmConfigsCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmConfigCommand(string PlatformAddress, string ConfigId);
