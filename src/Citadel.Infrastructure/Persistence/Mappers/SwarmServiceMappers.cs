using Domain;
using Domain.Entities.Deployments;
using Domain.Entities.SwarmServices;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class SwarmServiceMappers
{
    internal static IEnumerable<SwarmService> ToDomain(this IEnumerable<SwarmServiceDto> values) =>
        values.Select(static value => value.ToDomain());

    internal static SwarmService ToDomain(this SwarmServiceDto value)
    {
        var operation = value.OperationId is null
            ? null
            : new SwarmServiceOperation(
                value.OperationId.Value,
                Enum.Parse<SwarmServiceOperationKind>(value.OperationKind!),
                Enum.Parse<SwarmServiceOperationState>(value.OperationState!),
                value.BaseDockerVersion,
                value.TargetDesiredSpecHash!,
                value.TargetRuntimeHash,
                value.TargetRowVersion!.Value,
                value.ExpectedForceUpdate,
                value.PreparedAt!.Value,
                value.AttemptedAt,
                value.CompletedAt,
                value.ObservedDockerVersion,
                value.ResultCode,
                DeserializeWarnings(value.Warnings),
                value.ResultMessage,
                value.OperationClusterId,
                value.OperationActorId);

        var service = SwarmService.FromPersistence(
            value.Id,
            value.PlatformId,
            value.Name,
            value.Description,
            value.DockerName,
            value.DockerServiceId,
            JsonSerializer.Deserialize(value.Spec, SwarmServiceJsonContext.Default.SwarmServiceSpec)
                ?? throw new InvalidDataException($"Swarm Service {value.Id} has no valid specification."),
            new AutoUpdateState(
                value.AutoUpdateState_LastCheckedAt ?? DateTime.MinValue,
                string.IsNullOrWhiteSpace(value.AutoUpdateState_Status)
                    ? AutoUpdateStatus.Unknown
                    : Enum.Parse<AutoUpdateStatus>(value.AutoUpdateState_Status),
                value.AutoUpdateState_CurrentDigest,
                value.AutoUpdateState_RemoteDigest,
                value.AutoUpdateState_LastError),
            Enum.Parse<SwarmServiceHealth>(value.Health),
            Enum.Parse<SwarmServiceSynchronizationState>(value.SynchronizationState),
            Enum.Parse<ResourceControlState>(value.ControlState),
            value.ControlStartedAt,
            value.ControlTriggeredBy,
            value.DesiredSpecHash,
            value.LastAppliedDesiredSpecHash,
            value.LastAppliedRuntimeHash,
            value.AppliedImageDigest,
            value.DockerVersionIndex,
            operation,
            value.RowVersion,
            value.CreatedByActorId,
            value.CreatedAt,
            value.UpdatedAt,
            value.Platform_Descriptor is null || value.Platform_Name is null
                ? null
                : PlatformMappers.ToDomainSummary(
                    value.PlatformId,
                    value.Platform_Name,
                    value.Platform_Status,
                    value.Platform_Descriptor));

        service.AssignTags(value.TagsJson.ToTagSummaries());
        return service;
    }

    private static IReadOnlyList<string> DeserializeWarnings(string? value) =>
        string.IsNullOrWhiteSpace(value)
            ? []
            : JsonSerializer.Deserialize(value, PlatformJsonContext.Default.StringArray) ?? [];
}
