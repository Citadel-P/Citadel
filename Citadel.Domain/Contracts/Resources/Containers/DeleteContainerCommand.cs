namespace Domain.Contracts.Resources.Containers;

public sealed record DeleteContainerCommand(
    IReadOnlyDictionary<string, IEnumerable<string>> PlatformContainers,
    bool? Verbose,
    bool? Force,
    bool? Link);
