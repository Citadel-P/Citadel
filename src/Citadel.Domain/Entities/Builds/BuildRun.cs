namespace Domain.Entities.Builds;

public sealed class BuildRun(
    Guid buildProjectId,
    string projectNameSnapshot,
    Guid gitRepositoryId,
    string gitRepositoryNameSnapshot,
    string branch,
    string? resolvedCommitSha,
    string contextPath,
    string dockerfilePath,
    string? target,
    IReadOnlyList<BuildArgSpec> buildArgsSnapshot,
    IReadOnlyList<string> buildSecretIdsSnapshot,
    BuildPlatformSnapshot platformSnapshot,
    BuildRegistrySnapshot registrySnapshot,
    string imageRepository,
    IReadOnlyList<string> tagTemplatesSnapshot,
    IReadOnlyList<string> imageReferences,
    BuildRunTrigger trigger,
    Guid? triggerSourceId,
    Guid triggeredByActorId,
    int timeoutSeconds,
    DateTimeOffset? queuedAt = null,
    BuildRunStatus status = BuildRunStatus.Queued,
    string? imageDigest = null,
    DateTimeOffset? startedAt = null,
    DateTimeOffset? completedAt = null,
    int? exitCode = null,
    string? errorCode = null,
    string? errorMessage = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BuildProjectId { get; private set; } = buildProjectId;
    public string ProjectNameSnapshot { get; private set; } = BuildProject.NormalizeName(projectNameSnapshot);
    public Guid GitRepositoryId { get; private set; } = gitRepositoryId;
    public string GitRepositoryNameSnapshot { get; private set; } = BuildProject.NormalizeName(gitRepositoryNameSnapshot);
    public string Branch { get; private set; } = BuildProject.NormalizeName(branch);
    public string? ResolvedCommitSha { get; private set; } = BuildProject.NormalizeOptional(resolvedCommitSha);
    public string ContextPath { get; private set; } = contextPath;
    public string DockerfilePath { get; private set; } = dockerfilePath;
    public string? Target { get; private set; } = BuildProject.NormalizeOptional(target);
    public IReadOnlyList<BuildArgSpec> BuildArgsSnapshot { get; private set; } = buildArgsSnapshot;
    public IReadOnlyList<string> BuildSecretIdsSnapshot { get; private set; } = buildSecretIdsSnapshot;
    public BuildPlatformSnapshot PlatformSnapshot { get; private set; } = platformSnapshot;
    public BuildRegistrySnapshot RegistrySnapshot { get; private set; } = registrySnapshot;
    public string ImageRepository { get; private set; } = imageRepository;
    public IReadOnlyList<string> TagTemplatesSnapshot { get; private set; } = tagTemplatesSnapshot;
    public IReadOnlyList<string> ImageReferences { get; private set; } = imageReferences;
    public BuildRunTrigger Trigger { get; private set; } = trigger;
    public Guid? TriggerSourceId { get; private set; } = triggerSourceId;
    public BuildRunStatus Status { get; private set; } = status;
    public string? ImageDigest { get; private set; } = BuildProject.NormalizeOptional(imageDigest);
    public int TimeoutSeconds { get; private set; } = timeoutSeconds;
    public DateTimeOffset QueuedAt { get; private set; } = queuedAt?.ToUniversalTime() ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? StartedAt { get; private set; } = startedAt?.ToUniversalTime();
    public DateTimeOffset? CompletedAt { get; private set; } = completedAt?.ToUniversalTime();
    public int? ExitCode { get; private set; } = exitCode;
    public string? ErrorCode { get; private set; } = BuildProject.NormalizeOptional(errorCode);
    public string? ErrorMessage { get; private set; } = BuildProject.NormalizeOptional(errorMessage);
    public Guid TriggeredByActorId { get; private set; } = triggeredByActorId;

    public bool IsActive => Status is BuildRunStatus.Queued or BuildRunStatus.Preparing or BuildRunStatus.Running;

    public void MarkPreparing(DateTimeOffset now)
    {
        EnsureStatus(BuildRunStatus.Queued);
        Status = BuildRunStatus.Preparing;
        StartedAt ??= now.ToUniversalTime();
    }

    public void MarkRunning(DateTimeOffset now)
    {
        EnsureStatus(BuildRunStatus.Queued, BuildRunStatus.Preparing);
        Status = BuildRunStatus.Running;
        StartedAt ??= now.ToUniversalTime();
    }

    public void ResolveCommit(string commitSha, IReadOnlyList<string>? imageReferences = null)
    {
        ResolvedCommitSha = BuildProject.NormalizeOptional(commitSha);
        if (imageReferences is not null)
            ImageReferences = imageReferences;
    }

    public void CompleteSucceeded(string? imageDigest, IReadOnlyList<string> imageReferences, int? exitCode, DateTimeOffset now)
    {
        EnsureStatus(BuildRunStatus.Running, BuildRunStatus.Preparing);
        ImageDigest = BuildProject.NormalizeOptional(imageDigest);
        ImageReferences = imageReferences;
        Complete(BuildRunStatus.Succeeded, exitCode, null, null, now);
    }

    public void Fail(BuildRunStatus status, int? exitCode, string? errorCode, string errorMessage, DateTimeOffset now)
    {
        if (status is not (BuildRunStatus.Failed or BuildRunStatus.TimedOut or BuildRunStatus.Rejected or BuildRunStatus.Interrupted))
            throw new ArgumentException("Build run failure status is invalid.", nameof(status));

        Complete(status, exitCode, errorCode, errorMessage, now);
    }

    public void Cancel(DateTimeOffset now)
    {
        if (!IsActive)
            throw new InvalidOperationException($"Build run cannot be cancelled from status {Status}.");

        Complete(BuildRunStatus.Cancelled, null, "build.cancelled", "Build run cancelled.", now);
    }

    public static BuildRun FromPersistence(
        Guid id,
        Guid buildProjectId,
        string projectNameSnapshot,
        Guid gitRepositoryId,
        string gitRepositoryNameSnapshot,
        string branch,
        string? resolvedCommitSha,
        string contextPath,
        string dockerfilePath,
        string? target,
        IReadOnlyList<BuildArgSpec> buildArgsSnapshot,
        IReadOnlyList<string> buildSecretIdsSnapshot,
        BuildPlatformSnapshot platformSnapshot,
        BuildRegistrySnapshot registrySnapshot,
        string imageRepository,
        IReadOnlyList<string> tagTemplatesSnapshot,
        IReadOnlyList<string> imageReferences,
        BuildRunTrigger trigger,
        Guid? triggerSourceId,
        BuildRunStatus status,
        string? imageDigest,
        int timeoutSeconds,
        DateTimeOffset queuedAt,
        DateTimeOffset? startedAt,
        DateTimeOffset? completedAt,
        int? exitCode,
        string? errorCode,
        string? errorMessage,
        Guid triggeredByActorId)
        => new(
            buildProjectId,
            projectNameSnapshot,
            gitRepositoryId,
            gitRepositoryNameSnapshot,
            branch,
            resolvedCommitSha,
            contextPath,
            dockerfilePath,
            target,
            buildArgsSnapshot,
            buildSecretIdsSnapshot,
            platformSnapshot,
            registrySnapshot,
            imageRepository,
            tagTemplatesSnapshot,
            imageReferences,
            trigger,
            triggerSourceId,
            triggeredByActorId,
            timeoutSeconds,
            queuedAt,
            status,
            imageDigest,
            startedAt,
            completedAt,
            exitCode,
            errorCode,
            errorMessage)
        {
            Id = id
        };

    private void Complete(BuildRunStatus status, int? exitCode, string? errorCode, string? errorMessage, DateTimeOffset now)
    {
        Status = status;
        ExitCode = exitCode;
        ErrorCode = BuildProject.NormalizeOptional(errorCode);
        ErrorMessage = BuildProject.NormalizeOptional(errorMessage);
        CompletedAt = now.ToUniversalTime();
    }

    private void EnsureStatus(params BuildRunStatus[] statuses)
    {
        if (!statuses.Contains(Status))
            throw new InvalidOperationException($"Build run cannot transition from {Status}.");
    }
}
