using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class RoleRepository(IDbConnection db, Func<IDbTransaction> tx) : IRoleRepository
{
    public async Task<Role?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                r.Id,
                r.Name,
                p.Id AS PermissionId,
                p.ResourceType,
                p.ResourceAction
            FROM Roles r
            LEFT JOIN Permissions p ON p.RoleId = r.Id
            WHERE r.Id = @Id
            ORDER BY r.Name ASC
            """;
        var result = await db.QueryAsync<RolePermissionDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result.ToDomain().FirstOrDefault();
    }

    public async Task<IEnumerable<Role>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                r.Id,
                r.Name,
                p.Id AS PermissionId,
                p.ResourceType,
                p.ResourceAction
            FROM Roles r
            LEFT JOIN Permissions p ON p.RoleId = r.Id
            ORDER BY r.Name ASC
            """;
        var result = await db.QueryAsync<RolePermissionDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Role>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " " + """
            SELECT
                r.Id,
                r.Name,
                p.Id AS PermissionId,
                p.ResourceType,
                p.ResourceAction
            FROM Roles r
            LEFT JOIN Permissions p ON p.RoleId = r.Id
            WHERE
        """ + AuthorizationSql.ResourcePredicatePrefix + "r.Id" + AuthorizationSql.ResourcePredicateSuffix + " ORDER BY r.Name ASC;";

        var result = await db.QueryAsync<RolePermissionDto>(sql, new
        {
            UserId = userId,
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            cancellationToken
        }, transaction: tx());

        return result.ToDomain();
    }

    public async Task<IEnumerable<Role>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                r.Id,
                r.Name,
                p.Id AS PermissionId,
                p.ResourceType,
                p.ResourceAction
            FROM Roles r
            LEFT JOIN Permissions p ON p.RoleId = r.Id
            WHERE r.Id = ANY(@Ids)
            ORDER BY r.Name ASC
            """;
        var idArray = ids as Guid[] ?? [.. ids];
        var result = await db.QueryAsync<RolePermissionDto>(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Roles WHERE Name = @Name AND (@ExcludeId IS NULL OR Id != @ExcludeId))";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, ExcludeId = excludeId, cancellationToken }, transaction: tx());
    }

    public async Task<int> AddAsync(Role role, CancellationToken cancellationToken)
    {
        var sql = new StringBuilder("INSERT INTO Roles (Id, Name) VALUES (@Id, @Name);");
        var parameters = new DynamicParameters();
        parameters.Add("Id", role.Id);
        parameters.Add("Name", role.Name);

        using var enumerator = role.Permissions.GetEnumerator();
        if (enumerator.MoveNext())
        {
            sql.Append("INSERT INTO Permissions (Id, RoleId, ResourceType, ResourceAction) VALUES ");

            var index = 0;
            do
            {
                var permission = enumerator.Current;
                if (index > 0)
                    sql.Append(", ");

                sql.Append($"(@PermissionId{index}, @PermissionRoleId{index}, @ResourceType{index}, @ResourceAction{index})");
                parameters.Add($"PermissionId{index}", permission.Id);
                parameters.Add($"PermissionRoleId{index}", role.Id);
                parameters.Add($"ResourceType{index}", EnumFormatter<Hosting.Common.ResourceType>.GetValue(permission.ResourceType));
                parameters.Add($"ResourceAction{index}", EnumFormatter<Hosting.Common.ResourceAction>.GetValue(permission.ResourceAction));
                index++;
            }
            while (enumerator.MoveNext());

            sql.Append(';');
        }

        return await db.ExecuteAsync(sql.ToString(), parameters, transaction: tx());
    }

    public Task<int> RenameAsync(Role role, CancellationToken cancellationToken)
    {
        const string sql = "UPDATE Roles SET Name = @Name WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = role.Id, role.Name, cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Roles WHERE Id = ANY(@Ids)";
        var idArray = ids as Guid[] ?? [.. ids];
        return db.ExecuteAsync(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
    }

    public Task<IEnumerable<Guid>> GetActorRoleIdsAsync(Guid actorId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT RoleId FROM ActorRoles WHERE ActorId = @ActorId";
        return db.QueryAsync<Guid>(sql, new { ActorId = actorId, cancellationToken }, transaction: tx());
    }

    public Task<int> AddActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId) ON CONFLICT DO NOTHING";
        return db.ExecuteAsync(sql, new { ActorId = actorId, RoleId = roleId, cancellationToken }, transaction: tx());
    }

    public Task<int> RemoveActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM ActorRoles WHERE ActorId = @ActorId AND RoleId = @RoleId";
        return db.ExecuteAsync(sql, new { ActorId = actorId, RoleId = roleId, cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<Permission>> GetPermissionsAsync(Guid roleId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT ResourceType, ResourceAction FROM Permissions WHERE RoleId = @RoleId";
        var rows = await db.QueryAsync<PermissionAssignmentDto>(sql, new { RoleId = roleId, cancellationToken }, transaction: tx());
        return rows.Select(x => x.ToDomain(roleId));
    }

    public async Task<int> ReplaceActorRolesAsync(Guid actorId, IEnumerable<Guid> roleIds, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM ActorRoles WHERE ActorId = @ActorId";
        await db.ExecuteAsync(deleteSql, new { ActorId = actorId, cancellationToken }, transaction: tx());

        var rows = 0;
        foreach (var roleId in roleIds)
        {
            const string insertSql = "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId)";
            rows += await db.ExecuteAsync(insertSql, new { ActorId = actorId, RoleId = roleId, cancellationToken }, transaction: tx());
        }

        return rows;
    }

    public async Task<int> ReplacePermissionsAsync(Guid roleId, IEnumerable<Permission> permissions, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM Permissions WHERE RoleId = @RoleId";
        await db.ExecuteAsync(deleteSql, new { RoleId = roleId, cancellationToken }, transaction: tx());

        var rows = 0;
        foreach (var permission in permissions)
        {
            const string insertSql = "INSERT INTO Permissions (Id, RoleId, ResourceType, ResourceAction) VALUES (@Id, @RoleId, @ResourceType, @ResourceAction)";
            rows += await db.ExecuteAsync(insertSql, new
            {
                Id = permission.Id,
                RoleId = roleId,
                ResourceType = EnumFormatter<ResourceType>.GetValue(permission.ResourceType),
                ResourceAction = EnumFormatter<ResourceAction>.GetValue(permission.ResourceAction),
                cancellationToken
            }, transaction: tx());
        }

        return rows;
    }
}
