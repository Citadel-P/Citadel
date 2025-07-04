using Domain;

namespace Infrastructure.Persistence.Dtos;

internal sealed class UserDto
{
    public Guid Id { get; set; }
    public string Name { get; set; } = default!;
    public string Email { get; set; } = default!;
    public string? Password { get; set; }
    public IReadOnlyCollection<string> Roles { get; set; } = [];
    public IReadOnlyCollection<AppPermission> Permissions { get; set; } = [];
}
