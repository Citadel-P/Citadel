namespace Domain.Contracts.Resources.Containers;

public sealed record StreamContainersStatsCommand(string PlatformAddress, int FetchIntervalMs);