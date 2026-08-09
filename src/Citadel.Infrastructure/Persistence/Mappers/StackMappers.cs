using Domain;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class StackMappers
{
    internal static IEnumerable<Stack> ToDomain(this IEnumerable<StackDto> dtos)
        => dtos.Select(ToDomain);

    internal static Stack ToDomain(this StackDto dto)
    {
        var platform = dto.Platform_Name == null || dto.Platform_Descriptor == null
            ? null
            : PlatformMappers.ToDomainSummary(
                dto.CurrentRelease_PlatformId ?? Guid.Empty,
                dto.Platform_Name,
                dto.Platform_Status,
                dto.Platform_Descriptor);

        var currentRelease = !dto.HasCurrentReleaseIdentity
            ? null
            : StackRelease.FromPersistence(
                id: dto.CurrentRelease_Id!.Value,
                stackId: dto.CurrentRelease_StackId ?? dto.Id,
                platformId: dto.CurrentRelease_PlatformId!.Value,
                status: Enum.Parse<StackReleaseStatus>(dto.CurrentRelease_Status!),
                version: dto.CurrentRelease_Version!,
                spec: dto.CurrentRelease_Spec == null
                    ? new ManualStack(string.Empty, StackUpdateBehavior.Disabled)
                    : JsonSerializer.Deserialize(dto.CurrentRelease_Spec, StackJsonContext.Default.StackSpec)!,
                source: string.IsNullOrWhiteSpace(dto.CurrentRelease_Source)
                    ? null
                    : JsonSerializer.Deserialize(dto.CurrentRelease_Source, StackJsonContext.Default.StackReleaseSource),
                resourceBindings: string.IsNullOrWhiteSpace(dto.CurrentRelease_ResourceBindings)
                    ? null
                    : JsonSerializer.Deserialize(dto.CurrentRelease_ResourceBindings, StackJsonContext.Default.IReadOnlyListResourceBindingSnapshot),
                createdAt: dto.CurrentRelease_CreatedAt!.Value,
                createdByActorId: dto.CurrentRelease_CreatedByActorId!.Value,
                platform: platform);

        var latestActivityEvent = string.IsNullOrEmpty(dto.ActivityEvent_ActivityEventInfo) ? null : ActivityEvent.FromPersistence(
                id: dto.ActivityEvent_Id.Value,
                platformId: Guid.CreateVersion7(),
                resourceId: Guid.CreateVersion7(),
                resourceName: "N/A",
                createdAt: dto.ActivityEvent_CreatedAt.Value,
                createdByActorId: Guid.CreateVersion7(),
                resourceType: ActivityResourceType.Stack,
                info: JsonSerializer.Deserialize(dto.ActivityEvent_ActivityEventInfo, EventInfoJsonContext.Default.ActivityEventInfo),
                status: Enum.Parse<ActivityStatus>(dto.ActivityEvent_Status),
                eventType: Enum.Parse<ActivityEventType>(dto.ActivityEvent_EventType)
                );

        var stack = Stack.FromPersistence(
            id: dto.Id,
            currentStackReleaseId: dto.CurrentStackReleaseId,
            name: dto.Name,
            description: dto.Description,
            stackSource: Enum.Parse<StackSource>(dto.StackSource),
            stackUpdateState: JsonSerializer.Deserialize(dto.StackUpdateState, StackJsonContext.Default.StackUpdateState)!,
            driftPolicy: string.IsNullOrWhiteSpace(dto.DriftPolicy)
                ? StackDriftPolicy.Default
                : JsonSerializer.Deserialize(dto.DriftPolicy, StackJsonContext.Default.StackDriftPolicy) ?? StackDriftPolicy.Default,
            createdAt: dto.CreatedAt,
            createdByActorId: dto.CreatedByActorId,
            controlState: Enum.Parse<ResourceControlState>(dto.ControlState),
            controlTriggeredBy: dto.ControlTriggeredBy,
            controlStartedAt: dto.ControlStartedAt,
            rowVersion: dto.RowVersion,
            currentStackRelease: currentRelease,
            latestActivityEvent: latestActivityEvent);

        stack.AssignTags(dto.TagsJson.ToTagSummaries());
        return stack;
    }

    internal static IEnumerable<StackRelease> ToDomain(this IEnumerable<StackReleaseDto> dtos)
        => dtos.Select(ToDomain);

    internal static StackRelease ToDomain(this StackReleaseDto dto)
    {
        var platform = dto.Platform_Name == null
            ? null
            : Platform.FromPersistence(
                id: dto.PlatformId,
                name: dto.Platform_Name,
                address: string.Empty,
                networkCount: 0,
                volumeCount: 0,
                imageCount: 0,
                cpuCount: 0,
                memTotal: 0,
                status: dto.Platform_Status != null ? Enum.Parse<PlatformStatus>(dto.Platform_Status) : PlatformStatus.Offline,
                connectorType: PlatformConnectorType.Unknown,
                platformDescriptor: null);
        var actor = Actor.FromPersistence(
            id: dto.CreatedByActorId,
            type: dto.Actor_Type != null ? Enum.Parse<ActorType>(dto.Actor_Type) : ActorType.User,
            actorMetadata: new ActorMetadata(dto.CreatedByActorId == Constants.SystemId ? Constants.SystemName : dto.Actor_Name ?? "unknown"),
            isEnabled: true);

        return StackRelease.FromPersistence(
            id: dto.Id,
            stackId: dto.StackId,
            platformId: dto.PlatformId,
            status: Enum.Parse<StackReleaseStatus>(dto.Status),
            version: dto.Version,
            spec: JsonSerializer.Deserialize(dto.Spec, StackJsonContext.Default.StackSpec)!,
            source: string.IsNullOrWhiteSpace(dto.Source)
                ? null
                : JsonSerializer.Deserialize(dto.Source, StackJsonContext.Default.StackReleaseSource),
            resourceBindings: string.IsNullOrWhiteSpace(dto.ResourceBindings)
                ? null
                : JsonSerializer.Deserialize(dto.ResourceBindings, StackJsonContext.Default.IReadOnlyListResourceBindingSnapshot),
            createdAt: dto.CreatedAt,
            createdByActorId: dto.CreatedByActorId,
            platform: platform,
            actor: actor);
    }

    internal static StackReleaseVolumeBinding ToDomain(this StackReleaseVolumeBindingDto dto)
        => StackReleaseVolumeBinding.FromPersistence(
            dto.Id,
            dto.StackReleaseId,
            dto.PlatformId,
            dto.VolumeName,
            dto.ComposeVolumeName,
            dto.IsExternal,
            dto.IsAnonymous,
            new DateTimeOffset(DateTime.SpecifyKind(dto.CreatedAt, DateTimeKind.Utc)));

    internal static IEnumerable<StackReleaseVolumeBinding> ToDomain(this IEnumerable<StackReleaseVolumeBindingDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static StackReleaseSwarmResource ToDomain(this StackReleaseSwarmResourceDto dto)
        => StackReleaseSwarmResource.FromPersistence(
            dto.Id,
            dto.StackReleaseId,
            dto.PlatformId,
            Enum.Parse<StackReleaseSwarmResourceKind>(dto.Kind),
            dto.DockerResourceId,
            dto.DockerResourceName,
            dto.ComposeResourceName,
            JsonSerializer.Deserialize(dto.Mounts, StackJsonContext.Default.IReadOnlyListStackReleaseSwarmResourceMount));
}
