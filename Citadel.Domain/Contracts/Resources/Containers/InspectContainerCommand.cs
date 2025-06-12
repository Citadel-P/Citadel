namespace Domain.Contracts.Resources.Containers;

public sealed record InspectContainerCommand(
    string PlatformAddress,
    string ContainerId);
