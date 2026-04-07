using Domain.Entities.Identity;

namespace Domain.Contracts.Resources.Role;

public sealed record RoleDetails(Guid Id, string Name, IEnumerable<Permission> Permissions);
