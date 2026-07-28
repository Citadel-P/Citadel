namespace Domain.Entities.Stacks;

public enum StackWebhookDeployQueueStatus
{
    Queued,
    Processing
}

public sealed record StackWebhookDeployQueueItem(
    Guid Id,
    Guid StackId,
    Guid GitRepositoryId,
    Guid ExpectedStackReleaseId,
    string Branch,
    string ExpectedSpecFingerprint,
    string? DispatchedCommitSha,
    StackWebhookDeployQueueStatus Status,
    int Attempts,
    DateTime QueuedAt,
    DateTime AvailableAt,
    DateTime? StartedAt,
    string? LastError)
{
    public static StackWebhookDeployQueueItem Create(
        Guid stackId,
        Guid gitRepositoryId,
        Guid expectedStackReleaseId,
        string branch,
        string expectedSpecFingerprint,
        string? dispatchedCommitSha,
        DateTime queuedAt)
        => new(
            Guid.CreateVersion7(),
            stackId,
            gitRepositoryId,
            expectedStackReleaseId,
            branch,
            expectedSpecFingerprint,
            dispatchedCommitSha,
            StackWebhookDeployQueueStatus.Queued,
            0,
            queuedAt,
            queuedAt,
            null,
            null);
}
