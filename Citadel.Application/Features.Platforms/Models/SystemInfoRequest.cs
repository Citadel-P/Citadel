namespace Application.Features.Platforms.Models;

/// <summary>
/// The system info payload
/// </summary>
/// <param name="Id">The Docker daemon ID, it identifies the agent instance in Citadel Api</param>
/// <param name="Created">The datetime in which this message is sent (should be in UTC)</param>
/// <param name="MemoryUsage">the total memory used by all running containers</param>
/// <param name="CpuUsage">the total cpu used by all running containers</param>
/// <param name="RxBytes"></param>
/// <param name="TxBytes"></param>
/// <param name="NetworksCount">The networks count (only networks that are in use, dangling one's should not be included in the count)</param>
/// <param name="VolumesCount">The volumes count (only volumes that are in use, dangling one's should not be included in the count)</param>
/// <param name="AgentVersion"></param>
/// <param name="Containers"></param>
/// <param name="ContainersRunning"></param>
/// <param name="ContainersPaused"></param>
/// <param name="ContainersStopped"></param>
/// <param name="Images"></param>
/// <param name="Driver"></param>
/// <param name="DriverStatus"></param>
/// <param name="OperatingSystem"></param>
/// <param name="OsVersion"></param>
/// <param name="OsType"></param>
/// <param name="Architecture"></param>
/// <param name="Ncpu"></param>
/// <param name="MemTotal"></param>
/// <param name="ServerVersion"></param>
/// <param name="SwarmInfo"></param>
public sealed record SystemInfoRequest(
    string Id,
    long Created, 
    double MemoryUsage,
    double CpuUsage,
    double? RxBytes,
    double? TxBytes,
    short NetworksCount, 
    short VolumesCount,
    string AgentVersion, 
    long Containers,
    long ContainersRunning, 
    long ContainersPaused, 
    long ContainersStopped, 
    long Images, 
    string Driver,
    List<string[]> DriverStatus,
    string OperatingSystem, 
    string OsVersion,
    string OsType, 
    string Architecture, 
    long Ncpu, 
    long MemTotal,
    string ServerVersion, 
    SwarmInfoRequest SwarmInfo
    );

public sealed record SwarmInfoRequest(
    string NodeID,
    string NodeAddr,
    string LocalNodeState,
    bool ControlAvailable, 
    string Error, 
    List<SwarmPeerRequest> RemoteManagers,
    long Nodes,
    long Managers);

public sealed record SwarmPeerRequest(string NodeID, string Addr);
