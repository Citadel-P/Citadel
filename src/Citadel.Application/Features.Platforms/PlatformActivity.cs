using Domain;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;

namespace Application.Features.Platforms;

internal static class PlatformActivity
{
    public static ActivityEvent Created(Platform platform, Guid actorId)
        => Create(
            platform,
            actorId,
            ActivityEventType.PlatformCreated,
            ActivityStatus.Success,
            new PlatformCreated(platform.ToSnapshot()));

    public static ActivityEvent Deleted(Platform platform, Guid actorId)
        => Create(
            platform,
            actorId,
            ActivityEventType.PlatformDeleted,
            ActivityStatus.Success,
            new PlatformDeleted(platform.ToSnapshot()));

    public static ActivityEvent Connected(Platform platform, PlatformStatus previousStatus, Guid actorId)
        => Create(
            platform,
            actorId,
            ActivityEventType.PlatformConnected,
            ActivityStatus.Success,
            new PlatformConnected(platform.ToSnapshot(), previousStatus));

    public static ActivityEvent Disconnected(Platform platform, PlatformStatus previousStatus, Guid actorId)
        => Create(
            platform,
            actorId,
            ActivityEventType.PlatformDisconnected,
            ActivityStatus.Warning,
            new PlatformDisconnected(platform.ToSnapshot(), previousStatus));

    public static ActivityEvent NodeAgentLifecycle(
        Platform platform,
        Guid actorId,
        SwarmNodeAgentOperationKind kind,
        Guid operationId,
        SwarmNodeAgentOperationState state,
        string? message = null)
        => Create(
            platform,
            actorId,
            ActivityEventType.PlatformNodeAgentLifecycle,
            state switch
            {
                SwarmNodeAgentOperationState.Completed => ActivityStatus.Success,
                SwarmNodeAgentOperationState.Failed => ActivityStatus.Failure,
                _ => ActivityStatus.Information
            },
            new PlatformNodeAgentLifecycle(kind, operationId, state, message));

    private static ActivityEvent Create(
        Platform platform,
        Guid actorId,
        ActivityEventType eventType,
        ActivityStatus status,
        ActivityEventInfo info)
        => new(
            actorId: actorId,
            resourceId: platform.Id,
            platformId: platform.Id,
            resourceName: platform.Name,
            eventType: eventType,
            status: status,
            info: info);
}
