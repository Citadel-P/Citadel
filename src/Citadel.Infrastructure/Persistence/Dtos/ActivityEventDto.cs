namespace Infrastructure.Persistence.Dtos;

internal record ActivityEventDto(
    Guid Id,
    Guid? PlatformId,
    Guid? ResourceId,
    string ResourceName,
    string ResourceType,
    string EventType,
    string Info,
    Guid CreatedByActorId,
    DateTime CreatedAt
    );
