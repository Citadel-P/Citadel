namespace Domain.Configs;

public sealed class ServiceAccountOptions
{
    public const string SectionName = "ServiceAccounts";

    public int DefaultTokenLifetimeDays { get; init; } = 90;
    public int MaximumTokenLifetimeDays { get; init; } = 365;
    public int MaximumActiveTokensPerAccount { get; init; } = 10;
    public int LastUsedWriteIntervalMinutes { get; init; } = 5;
    public int LastUsedTrackingCapacity { get; init; } = 10_000;
}
