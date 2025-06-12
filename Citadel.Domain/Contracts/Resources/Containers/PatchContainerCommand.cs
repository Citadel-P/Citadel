namespace Domain.Contracts.Resources.Containers;

public sealed record PatchContainerCommand(
    ContainerAction Action,
    IReadOnlyDictionary<string, IEnumerable<string>> PlatformContainers);
