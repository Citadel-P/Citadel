namespace Infrastructure.Persistence.Dtos;

internal sealed record TeamRoleAssignmentStateDto(
    Guid? Id,
    string? Name,
    Guid? ActorId,
    bool? IsEnabled,
    bool RoleExists);
