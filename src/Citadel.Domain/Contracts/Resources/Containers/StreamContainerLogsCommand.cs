namespace Domain.Contracts.Resources.Containers;

public sealed record StreamContainerLogsCommand(string PlatformAddress, string ContainerId);
