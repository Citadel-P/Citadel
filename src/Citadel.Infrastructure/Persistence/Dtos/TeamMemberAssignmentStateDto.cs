namespace Infrastructure.Persistence.Dtos;

internal sealed record TeamMemberAssignmentStateDto(
    Guid? Id,
    string? Name,
    Guid? ActorId,
    bool? IsEnabled,
    int TotalMembers,
    IEnumerable<string> Roles,
    bool UserExists,
    bool HasMember);
