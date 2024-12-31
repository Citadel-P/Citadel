namespace Application.Features.Containers.Models;

/// <summary>
/// Represents the payload of all containers in a Docker daemon
/// </summary>
/// <param name="Id">The Docker daemon ID, it identifies the agent instance in Citadel Api</param>
/// <param name="Created">The datetime in which this message is sent</param>
/// <param name="ContainersInfo"></param>
public sealed record ContainersInfoRequest (string Id, long Created, List<ContainerInfoRequest> ContainersInfo);

public sealed record ContainerInfoRequest (
    string Id,
    string Image, 
    string ImageID, 
    string Command, 
    int Created, 
    long SizeRw,
    string Status,
    string State,
    long SizeRootFs, 
    List<string> Names,
    List<PortRequest> Ports,
    ContainerStatRequest ContainerStat,
    IDictionary<string, string> Labels);

/// <summary>
/// 
/// </summary>
/// <param name="MemoryUsage">The total memory used by the container</param>
/// <param name="CpuUsage">The total cpu used by container</param>
/// <param name="MemoryLimit">The memory limit of the container</param>
/// <param name="RxBytes">Total received bytes</param>
/// <param name="TxBytes">Total transmitted bytes</param>
public sealed record ContainerStatRequest(
    double MemoryUsage,
    double CpuUsage,
    double MemoryLimit,
    ulong RxBytes,
    ulong TxBytes);

public sealed record PortRequest(string IP, ushort PrivatePort, ushort PublicPort, string Type);