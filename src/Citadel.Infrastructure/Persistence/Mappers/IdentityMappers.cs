using Domain;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class IdentityMappers
{
    private static readonly IEnumerable<ResourceInfo> EmptyResources = [];

    internal static User ToDomain(this UserDto dto)
        => User.FromPersistence(dto.Id, dto.Name, dto.Email, dto.Password, dto.ActorId, dto.CreatedByActorId, dto.CreatedAt);

    internal static IEnumerable<User> ToDomain(this IEnumerable<UserDto> dtos)
        => dtos.Select(ToDomain);

    internal static UserDetails ToDetails(this UserWithActorDto dto)
        => new(dto.Id, dto.Name, dto.Email, dto.ActorId, dto.IsEnabled, dto.CreatedAt, dto.CreatedByActorId, ParseResources(dto.Teams), ParseResources(dto.Roles));

    internal static IEnumerable<UserDetails> ToDetails(this IEnumerable<UserWithActorDto> dtos)
        => dtos.Select(ToDetails);

    private static IEnumerable<ResourceInfo> ParseResources(string json)
        => JsonSerializer.Deserialize(json, RoleJsonContext.Default.IEnumerableResourceInfo) ?? EmptyResources;

    internal static TeamDetails ToDetails(this TeamWithActorDto dto)
        => new(dto.Id, dto.Name, dto.ActorId, dto.IsEnabled, dto.TotalMembers, ParseResources(dto.Roles));

    internal static IEnumerable<TeamDetails> ToDetails(this IEnumerable<TeamWithActorDto> dtos)
        => dtos.Select(ToDetails);

    internal static Team ToDomain(this TeamDto dto)
        => Team.FromPersistence(dto.Id, dto.Name, dto.ActorId);

    internal static IEnumerable<Team> ToDomain(this IEnumerable<TeamDto> dtos)
        => dtos.Select(ToDomain);

    internal static Role ToDomain(this RoleDto dto)
        => Role.FromPersistence(dto.Id, dto.Name, Enum.Parse<Domain.RoleType>(dto.RoleType));

    internal static IEnumerable<Role> ToDomain(this IEnumerable<RoleDto> dtos)
        => dtos.Select(ToDomain);

    internal static Permission ToDomain(this PermissionAssignmentDto dto, Guid roleId)
        => Permission.Create(
            roleId,
            (ResourceType)dto.ResourceType,
            (PermissionLevel)dto.PermissionLevel,
            Permission.FromSpecificPermissionsMask(dto.SpecificPermissions));

    internal static IEnumerable<Role> ToDomain(this IEnumerable<RolePermissionDto> dtos)
        => dtos
            .GroupBy(x => new { x.Id, x.Name, x.RoleType })
            .Select(group => Role.FromPersistence(
                group.Key.Id,
                group.Key.Name,
                Enum.Parse<RoleType>(group.Key.RoleType),
                group.Where(x => x.PermissionId.HasValue && x.ResourceType.HasValue && x.PermissionLevel.HasValue)
                    .Select(x => Permission.Create(
                        group.Key.Id,
                        (ResourceType)x.ResourceType!.Value,
                        (PermissionLevel)x.PermissionLevel!.Value,
                        Permission.FromSpecificPermissionsMask(x.SpecificPermissions ?? 0),
                        x.PermissionId!.Value))));
}
