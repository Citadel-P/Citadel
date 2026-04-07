namespace Infrastructure.Persistence.Dtos;

internal sealed record TeamMemberAssignmentStateDto(
    Guid? Id,
    string? Name,
    Guid? ActorId,
    bool? IsEnabled,
    bool UserExists,
    bool HasMember);
