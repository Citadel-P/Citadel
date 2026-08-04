using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;

namespace Domain.Contracts.Resources.Containers;

public enum DaemonEventScope
{
    Unknown = 0,
    Local,
    Swarm
}

public abstract record DaemonEventInfo(
    string Action,
    DaemonEventScope Scope = DaemonEventScope.Unknown);

public sealed record DaemonContainerEventInfo(
    string Action,
    string ContainerId,
    DockerContainer? Container,
    DaemonEventScope Scope = DaemonEventScope.Unknown) : DaemonEventInfo(Action, Scope);

public sealed record DaemonImageEventInfo(
    string Action,
    string ImageId,
    ImageResult? Image,
    DaemonEventScope Scope = DaemonEventScope.Unknown) : DaemonEventInfo(Action, Scope);

public sealed record DaemonVolumeEventInfo(
    string Action,
    string VolumeId,
    DockerVolumeResult? Volume,
    DaemonEventScope Scope = DaemonEventScope.Unknown) : DaemonEventInfo(Action, Scope);

public sealed record DaemonNetworkEventInfo(
    string Action,
    string NetworkId,
    DockerNetworkResult? Network,
    DaemonEventScope Scope = DaemonEventScope.Unknown) : DaemonEventInfo(Action, Scope);

public sealed record DaemonResourceEventInfo(
    string Action,
    ContainerEventType Type,
    string ResourceId,
    DaemonEventScope Scope = DaemonEventScope.Unknown) : DaemonEventInfo(Action, Scope);
