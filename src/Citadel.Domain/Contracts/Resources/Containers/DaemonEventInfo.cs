using Domain.Contracts.Resources.Images;

namespace Domain.Contracts.Resources.Containers;

public abstract record DaemonEventInfo(string Action);

public record DaemonContainerEventInfo(
    string Action,
    string ContainerId,
    DockerContainer? Container) : DaemonEventInfo (Action);

public record DaemonImageEventInfo(
    string Action,
    string ImageId,
    ImageResult Image) : DaemonEventInfo(Action);