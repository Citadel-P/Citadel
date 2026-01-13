using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;

namespace Domain.Contracts.Resources.Containers;

public abstract record DaemonEventInfo(string Action);

public sealed record DaemonContainerEventInfo(
    string Action,
    string ContainerId,
    DockerContainer? Container) : DaemonEventInfo (Action);

public sealed record DaemonImageEventInfo(
    string Action,
    string ImageId,
    ImageResult? Image) : DaemonEventInfo(Action);

public sealed record DaemonVolumeEventInfo(
    string Action,
    string VolumeId,
    DockerVolumeResult? Volume) : DaemonEventInfo(Action);

public sealed record DaemonNetworkEventInfo(
    string Action,
    string NetworkId,
    DockerNetworkResult? Network) : DaemonEventInfo(Action);