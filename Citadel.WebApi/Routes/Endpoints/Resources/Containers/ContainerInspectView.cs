using Citadel.Common;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerInspectView(
    string Id,
    string Created,
    string Path,
    ICollection<string> Args,
    ContainerState State,
    string Image,
    string ResolvConfPath,
    string HostnamePath,
    string HostsPath,
    string LogPath,
    string Name,
    int? RestartCount,
    string Driver,
    string Platform,
    string MountLabel,
    string ProcessLabel,
    string AppArmorProfile,
    ICollection<string> ExecIDs,
    HostConfig HostConfig,
    GraphDriverData GraphDriver,
    long? SizeRw,
    long? SizeRootFs,
    ICollection<MountPoint> Mounts,
    ContainerConfig Config,
    NetworkSettingsInspectView NetworkSettings)
{
    internal static ContainerInspectView Map(ContainerInspectReply containerInspect) 
        => containerInspect.Map();
}

public record NetworkSettingsInspectView(
        string Bridge,
        string SandboxID,
        bool? HairpinMode,
        string LinkLocalIPv6Address,
        int? LinkLocalIPv6PrefixLen,
        IEnumerable<MapFieldPortBindingView> Ports,
        string SandboxKey,
        IEnumerable<Address> SecondaryIPAddresses,
        string EndpointID,
        string Gateway,
        string GlobalIPv6Address,
        int? GlobalIPv6PrefixLen,
        string IPAddress,
        int? IPPrefixLen,
        string IPv6Gateway,
        string MacAddress,
        Dictionary<string, EndpointSettings> Networks
    );

public record MapFieldPortBindingView(string Key, IEnumerable<PortBindingView> Value);

public record PortBindingView(string HostIP, string HostPort);