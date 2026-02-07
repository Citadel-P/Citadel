namespace Infrastructure.Persistence.Dtos;

internal record ActivityEventDto(
    Guid Id,
    Guid? PlatformId,
    Guid? ResourceId,
    string ResourceName,
    string ResourceType,
    string EventType,
    string Status,
    string Info,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    string? Platform_Name = null,
    string? Platform_Status= null,
    string? Actor_Name = null,
    string? Actor_Type = null
    );
