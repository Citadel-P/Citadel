namespace Domain.Contracts.Resources.Containers;

public sealed record StreamContainerStatsCommand(string PlatformAddress, int FetchIntervalMs);