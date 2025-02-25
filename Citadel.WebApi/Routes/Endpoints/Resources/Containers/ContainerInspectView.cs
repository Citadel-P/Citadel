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
    NetworkSettingsView NetworkSettings)
{
    internal static ContainerInspectView Map(ContainerInspectReply containerInspect) 
        => containerInspect.Map();
}

