using Domain;
using Domain.Entities.Builds;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class BuildMappers
{
    internal static IEnumerable<BuildProject> ToDomain(this IEnumerable<BuildProjectDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BuildProject ToDomain(this BuildProjectDto dto)
    {
        var project = BuildProject.FromPersistence(
            dto.Id,
            dto.Name,
            dto.NormalizedName,
            dto.Description,
            dto.Enabled,
            dto.GitRepositoryId,
            dto.Branch,
            dto.ContextPath,
            dto.DockerfilePath,
            dto.Target,
            DeserializeBuildArgs(dto.BuildArgs),
            DeserializeBuildSecrets(dto.BuildSecrets),
            dto.PlatformId,
            dto.RegistryId,
            dto.ImageRepository,
            DeserializeStringList(dto.TagTemplates),
            DeserializeWebhook(dto.Webhook),
            dto.TimeoutSeconds,
            dto.RetentionRunCount,
            dto.CurrentRunId,
            string.IsNullOrWhiteSpace(dto.ControlState)
                ? ResourceControlState.Idle
                : Enum.Parse<ResourceControlState>(dto.ControlState),
            dto.ControlStartedAt,
            dto.CreatedByActorId,
            ToOffset(dto.CreatedAt),
            ToOffset(dto.UpdatedAt),
            ToOffset(dto.ArchivedAt),
            dto.RowVersion);

        project.AssignTags(dto.TagsJson.ToTagSummaries());
        return project;
    }

    internal static IEnumerable<BuildRun> ToDomain(this IEnumerable<BuildRunDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BuildRun ToDomain(this BuildRunDto dto)
        => BuildRun.FromPersistence(
            dto.Id,
            dto.BuildProjectId,
            dto.ProjectNameSnapshot,
            dto.GitRepositoryId,
            dto.GitRepositoryNameSnapshot,
            dto.Branch,
            dto.ResolvedCommitSha,
            dto.ContextPath,
            dto.DockerfilePath,
            dto.Target,
            DeserializeBuildArgs(dto.BuildArgsSnapshot),
            DeserializeStringList(dto.BuildSecretIdsSnapshot),
            DeserializePlatformSnapshot(dto.PlatformSnapshot),
            DeserializeRegistrySnapshot(dto.RegistrySnapshot),
            dto.ImageRepository,
            DeserializeStringList(dto.TagTemplatesSnapshot),
            DeserializeStringList(dto.ImageReferences),
            Enum.Parse<BuildRunTrigger>(dto.Trigger),
            dto.TriggerSourceId,
            Enum.Parse<BuildRunStatus>(dto.Status),
            dto.ImageDigest,
            dto.TimeoutSeconds,
            ToOffset(dto.QueuedAt),
            ToOffset(dto.StartedAt),
            ToOffset(dto.CompletedAt),
            dto.ExitCode,
            dto.ErrorCode,
            dto.ErrorMessage,
            dto.TriggeredByActorId);

    internal static IEnumerable<BuildRunLogEntry> ToDomain(this IEnumerable<BuildRunLogDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BuildRunLogEntry ToDomain(this BuildRunLogDto dto)
        => new(dto.Id, dto.BuildRunId, ToOffset(dto.CreatedAt), dto.Stream, dto.Message);

    internal static string SerializeBuildArgs(IReadOnlyList<BuildArgSpec> values)
        => JsonSerializer.Serialize(values, BuildJsonContext.Default.IReadOnlyListBuildArgSpec);

    internal static string SerializeBuildSecrets(IReadOnlyList<BuildSecretSpec> values)
        => JsonSerializer.Serialize(values, BuildJsonContext.Default.IReadOnlyListBuildSecretSpec);

    internal static string SerializeStringList(IReadOnlyList<string> values)
        => JsonSerializer.Serialize(values, BuildJsonContext.Default.IReadOnlyListString);

    internal static string SerializePlatformSnapshot(BuildPlatformSnapshot value)
        => JsonSerializer.Serialize(value, BuildJsonContext.Default.BuildPlatformSnapshot);

    internal static string SerializeRegistrySnapshot(BuildRegistrySnapshot value)
        => JsonSerializer.Serialize(value, BuildJsonContext.Default.BuildRegistrySnapshot);

    internal static string? SerializeWebhook(BuildWebhookConfig? value)
        => value is null
            ? null
            : JsonSerializer.Serialize(value, BuildJsonContext.Default.BuildWebhookConfig);

    private static IReadOnlyList<BuildArgSpec> DeserializeBuildArgs(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? []
            : JsonSerializer.Deserialize(json, BuildJsonContext.Default.IReadOnlyListBuildArgSpec) ?? [];

    private static IReadOnlyList<BuildSecretSpec> DeserializeBuildSecrets(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? []
            : JsonSerializer.Deserialize(json, BuildJsonContext.Default.IReadOnlyListBuildSecretSpec) ?? [];

    private static IReadOnlyList<string> DeserializeStringList(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? []
            : JsonSerializer.Deserialize(json, BuildJsonContext.Default.IReadOnlyListString) ?? [];

    private static BuildPlatformSnapshot DeserializePlatformSnapshot(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? new BuildPlatformSnapshot(Guid.Empty, string.Empty, string.Empty, PlatformConnectorType.Local)
            : JsonSerializer.Deserialize(json, BuildJsonContext.Default.BuildPlatformSnapshot)
              ?? new BuildPlatformSnapshot(Guid.Empty, string.Empty, string.Empty, PlatformConnectorType.Local);

    private static BuildRegistrySnapshot DeserializeRegistrySnapshot(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? new BuildRegistrySnapshot(Guid.Empty, string.Empty, string.Empty)
            : JsonSerializer.Deserialize(json, BuildJsonContext.Default.BuildRegistrySnapshot)
              ?? new BuildRegistrySnapshot(Guid.Empty, string.Empty, string.Empty);

    private static BuildWebhookConfig? DeserializeWebhook(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? null
            : JsonSerializer.Deserialize(json, BuildJsonContext.Default.BuildWebhookConfig);

    internal static DateTimeOffset ToOffset(DateTime value)
        => new(DateTime.SpecifyKind(value, DateTimeKind.Utc));

    internal static DateTimeOffset? ToOffset(DateTime? value)
        => value.HasValue ? ToOffset(value.Value) : null;
}
