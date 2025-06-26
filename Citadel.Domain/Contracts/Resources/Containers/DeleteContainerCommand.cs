namespace Domain.Contracts.Resources.Containers;

public sealed record DeleteContainerCommand(
    IEnumerable<string> ContainerIds,
    string PlatformAddress,
    bool? Volume,
    bool? Force,
    bool? Link);
