namespace Domain.Contracts.Resources.Containers;

public sealed record DeleteContainerCommand(
    IEnumerable<string> ContainerIds,
    string PlatformAddress,
    bool? Verbose,
    bool? Force,
    bool? Link);
