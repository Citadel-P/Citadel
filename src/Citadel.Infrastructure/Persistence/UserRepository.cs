using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence;

internal sealed class UserRepository(IDbConnection db, Func<IDbTransaction> tx) : IUserRepository
{
    public async Task<UserAuthInfo?> GetUserAuthInfoByEmailAsync(string email, CancellationToken cancellationToken)
    {
        const string sql =
            """
            WITH TargetUser AS (
                SELECT *
                FROM Users
                WHERE Email = @Email
                LIMIT 1
            )
            SELECT 
                TargetUser.Id, 
                TargetUser.Name, 
                TargetUser.Email,
                TargetUser.Password,
                Roles.Name as RoleName, 
                Permissions.PermissionCode
            FROM TargetUser
            LEFT JOIN UsersTeams ON TargetUser.Id = UsersTeams.UserId
            LEFT JOIN Teams ON Teams.Id = UsersTeams.TeamId
            LEFT JOIN Roles ON Teams.RoleId = Roles.Id
            LEFT JOIN Permissions ON Roles.Id = Permissions.RoleId
            """;

        var result = await db.QueryAsync<UserAuthInfoDto>(sql,
            new { Email = email },
            transaction: tx());

        return result
            .GroupBy(r => new { r.Id, r.Name, r.Email, r.Password })
            .Select(g => new UserAuthInfo(
                g.Key.Id,
                g.Key.Name,
                g.Key.Email,
                g.Key.Password,
                [.. g.Where(r => !string.IsNullOrWhiteSpace(r.RoleName))
                    .Select(r => r.RoleName!)
                    .Distinct()],
                [.. g.Where(r => r.PermissionCode.HasValue)
                    .Select(r => (AppPermission)r.PermissionCode!.Value)
                    .Distinct()]
            )).FirstOrDefault();
    }
}
