namespace Domain.Contracts.Resources.Swarm;

using Domain.Entities.SwarmServices;

public static class SwarmInventoryLimits
{
    public const int MaximumItems = 500;
    public const int AuthoritativeSnapshotItems = int.MaxValue;
    public const int DefaultLogLines = 100;
    public const int MaximumLogLines = 200;

    public static int Normalize(int requested) => Math.Clamp(requested, 1, MaximumItems);
    public static int NormalizeConnectorLimit(int requested) =>
        requested == AuthoritativeSnapshotItems ? AuthoritativeSnapshotItems : Normalize(requested);
    public static int NormalizeLogLines(int requested) => Math.Clamp(requested, 1, MaximumLogLines);
}

public sealed record ListSwarmServicesCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record UpdateSwarmNodeCommand(
    string PlatformAddress,
    string NodeId,
    long VersionIndex,
    string Availability,
    IReadOnlyDictionary<string, string> Labels);
public sealed record InspectSwarmServiceCommand(string PlatformAddress, string ServiceId);
public sealed record RestartSwarmServiceCommand(string PlatformAddress, string ServiceId);
public sealed record DeleteSwarmInventoryServiceCommand(string PlatformAddress, string ServiceId);
public sealed record GetSwarmServiceLogsCommand(
    string PlatformAddress,
    string ServiceId,
    int Tail = SwarmInventoryLimits.DefaultLogLines);
public sealed record ListSwarmTasksCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmTaskCommand(string PlatformAddress, string TaskId);
public sealed record GetSwarmTaskLogsCommand(
    string PlatformAddress,
    string TaskId,
    int Tail = SwarmInventoryLimits.DefaultLogLines);
public sealed record ListSwarmNetworksCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmNetworkCommand(string PlatformAddress, string NetworkId);
public sealed record ListSwarmSecretsCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmSecretCommand(string PlatformAddress, string SecretId);
public sealed record CreateSwarmSecretCommand(
    string PlatformAddress,
    string Name,
    byte[] Data,
    IReadOnlyDictionary<string, string> Labels);
public sealed record UpdateSwarmSecretLabelsCommand(
    string PlatformAddress,
    string SecretId,
    long VersionIndex,
    IReadOnlyDictionary<string, string> Labels);
public sealed record DeleteSwarmSecretCommand(string PlatformAddress, string SecretId);
public sealed record ListSwarmConfigsCommand(string PlatformAddress, int Limit = SwarmInventoryLimits.MaximumItems);
public sealed record InspectSwarmConfigCommand(string PlatformAddress, string ConfigId);
public sealed record CreateSwarmConfigCommand(
    string PlatformAddress,
    string Name,
    byte[] Data,
    IReadOnlyDictionary<string, string> Labels);
public sealed record UpdateSwarmConfigLabelsCommand(
    string PlatformAddress,
    string ConfigId,
    long VersionIndex,
    IReadOnlyDictionary<string, string> Labels);
public sealed record DeleteSwarmConfigCommand(string PlatformAddress, string ConfigId);

public sealed record SwarmResourceCreationResult(string ResourceId);

public sealed record SystemSwarmServiceSpec(
    string Image,
    IReadOnlyList<string> Environment,
    string ManagerNodeId,
    string StateVolumeName,
    string BootstrapSecretId,
    string BootstrapSecretName,
    string? CaConfigId,
    string? CaConfigName,
    long LimitNanoCpus,
    long LimitMemoryBytes,
    long PidsLimit,
    long StopGracePeriodNanoseconds,
    IReadOnlyList<string> SupportedArchitectures);

public sealed record CreateSystemSwarmServiceCommand(
    string PlatformAddress,
    Guid OperationId,
    string DockerName,
    SystemSwarmServiceSpec Spec,
    IReadOnlyDictionary<string, string> Labels,
    IReadOnlyDictionary<string, string> ContainerLabels);

public sealed record UpdateSystemSwarmServiceCommand(
    string PlatformAddress,
    Guid OperationId,
    string ServiceId,
    long VersionIndex,
    SystemSwarmServiceSpec Spec,
    IReadOnlyDictionary<string, string> Labels,
    IReadOnlyDictionary<string, string> ContainerLabels);

public sealed record CreateManagedSwarmServiceCommand(
    string PlatformAddress,
    Guid OperationId,
    string DockerName,
    SwarmServiceSpec Spec,
    string ResolvedImage,
    IReadOnlyDictionary<string, string> Labels,
    string? RegistryAuth);

public sealed record UpdateManagedSwarmServiceCommand(
    string PlatformAddress,
    Guid OperationId,
    string ServiceId,
    long VersionIndex,
    SwarmServiceSpec Spec,
    string ResolvedImage,
    IReadOnlyDictionary<string, string> Labels,
    string? RegistryAuth,
    int ForceUpdate = 0);

public sealed record DeleteManagedSwarmServiceCommand(
    string PlatformAddress,
    Guid OperationId,
    string ServiceId);

public sealed record ManagedSwarmServiceMutationResult(
    string? ServiceId,
    IReadOnlyList<string> Warnings);
