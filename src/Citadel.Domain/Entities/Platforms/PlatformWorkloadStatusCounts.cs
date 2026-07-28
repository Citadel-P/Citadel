namespace Domain.Entities.Platforms;

public sealed record PlatformWorkloadStatusCounts(
    long Total,
    long Healthy,
    long Degraded,
    long Failed,
    long Stopped,
    long Paused,
    long InProgress,
    long Unknown)
{
    public static PlatformWorkloadStatusCounts Empty { get; } = new(0, 0, 0, 0, 0, 0, 0, 0);
}
