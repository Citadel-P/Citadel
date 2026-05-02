namespace Infrastructure.Persistence.Dtos;

internal sealed record RoleDto(
    Guid Id,
    string Name,
    string RoleType);
