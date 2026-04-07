using Domain.Entities.Identity;
using Domain.Contracts.Resources.Identity;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class IdentityMappers
{
    internal static User ToDomain(this UserDto dto)
        => User.FromPersistence(dto.Id, dto.Name, dto.Email, dto.Password, dto.ActorId, dto.CreatedByActorId, dto.CreatedAt);

    internal static IEnumerable<User> ToDomain(this IEnumerable<UserDto> dtos)
        => dtos.Select(ToDomain);

    internal static UserDetails ToDetails(this UserWithActorDto dto)
        => new(dto.Id, dto.Name, dto.Email, dto.ActorId, dto.IsEnabled, dto.CreatedAt, dto.CreatedByActorId);

    internal static IEnumerable<UserDetails> ToDetails(this IEnumerable<UserWithActorDto> dtos)
        => dtos.Select(ToDetails);

    internal static Team ToDomain(this TeamDto dto)
        => Team.FromPersistence(dto.Id, dto.Name, dto.ActorId);

    internal static IEnumerable<Team> ToDomain(this IEnumerable<TeamDto> dtos)
        => dtos.Select(ToDomain);

    internal static Role ToDomain(this RoleDto dto)
        => Role.FromPersistence(dto.Id, dto.Name);

    internal static IEnumerable<Role> ToDomain(this IEnumerable<RoleDto> dtos)
        => dtos.Select(ToDomain);

    internal static Permission ToDomain(this PermissionAssignmentDto dto, Guid roleId)
        => Permission.Create(roleId, Enum.Parse<Hosting.Common.ResourceType>(dto.ResourceType), Enum.Parse<Hosting.Common.ResourceAction>(dto.ResourceAction));

    internal static IEnumerable<Role> ToDomain(this IEnumerable<RolePermissionDto> dtos)
        => dtos
            .GroupBy(x => new { x.Id, x.Name })
            .Select(group => Role.FromPersistence(
                group.Key.Id,
                group.Key.Name,
                group.Where(x => x.PermissionId.HasValue && x.ResourceType is not null && x.ResourceAction is not null)
                    .Select(x => Permission.Create(
                        group.Key.Id,
                        Enum.Parse<Hosting.Common.ResourceType>(x.ResourceType!),
                        Enum.Parse<Hosting.Common.ResourceAction>(x.ResourceAction!),
                        x.PermissionId!.Value))));
}
