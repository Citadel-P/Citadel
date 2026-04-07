namespace Infrastructure.Persistence.Dtos;

internal sealed record UserRoleAssignmentStateDto(
    Guid? Id,
    string? Name,
    string? Email,
    Guid? ActorId,
    bool? IsEnabled,
    DateTime? CreatedAt,
    Guid? CreatedByActorId,
    bool RoleExists,
    bool HasRole);
