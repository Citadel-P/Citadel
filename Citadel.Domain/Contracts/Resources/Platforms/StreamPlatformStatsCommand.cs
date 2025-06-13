namespace Domain.Contracts.Resources.Platforms;

public sealed record StreamPlatformStatsCommand(string PlatformAddress, Guid PlatformId, int FetchIntervalMs = 10_000);
