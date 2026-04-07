namespace Infrastructure.Persistence.Dtos;

internal sealed record PermissionAssignmentDto(
    string ResourceType,
    string ResourceAction);
