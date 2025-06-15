namespace Domain.Contracts.Resources.Containers;

public sealed record PatchContainerCommand(
    ContainerAction Action,
    string PlatformAddress,
    IEnumerable<string> ContainerIds);
