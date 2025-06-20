namespace Domain.Contracts.Resources.Platforms;

public sealed record StreamPlatformStatsCommand(string PlatformAddress, int FetchIntervalMs = 10_000);
