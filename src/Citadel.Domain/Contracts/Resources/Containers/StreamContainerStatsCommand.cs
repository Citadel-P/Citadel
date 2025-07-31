namespace Domain.Contracts.Resources.Containers;

public sealed record StreamContainerStatsCommand(string ContainerId, string PlatformAddress, int FetchIntervalMs);