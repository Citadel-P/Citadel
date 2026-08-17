using System.Text.Json;
using System.Text.Json.Serialization;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
using Microsoft.Extensions.DependencyInjection;
using Npgsql;

namespace Tests.Integration.Application.Features.Permissions;

public sealed class AuthorizationDifferentialTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly JsonSerializerOptions SerializerOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        Converters = { new JsonStringEnumConverter() }
    };

    [Fact]
    public async Task PermissionMatrix_ShouldMatchSharedRustContract()
    {
        var matrix = await LoadMatrixAsync();
        var actual = PermissionMatrix.GetAll();

        Assert.Equal(matrix.PermissionMatrix.Count, actual.Count);
        foreach (var (resourceType, expected) in matrix.PermissionMatrix)
        {
            Assert.True(actual.TryGetValue(resourceType, out var capability), $"Missing {resourceType}.");
            Assert.Equal(expected.MaximumLevel, capability.MaximumLevel);
            Assert.Equal(expected.Specifics.Count, capability.SpecificPermissionMinimumLevels.Count);
            foreach (var (specific, minimumLevel) in expected.Specifics)
            {
                Assert.True(
                    capability.SpecificPermissionMinimumLevels.TryGetValue(specific, out var actualMinimum),
                    $"Missing {resourceType}/{specific}.");
                Assert.Equal(minimumLevel, actualMinimum);
            }
        }
    }

    [Fact]
    public async Task PostgreSqlAuthorization_ShouldMatchSharedRustMatrix()
    {
        var matrix = await LoadMatrixAsync();

        Assert.Equal(1, matrix.SchemaVersion);
        foreach (var authorizationCase in matrix.Cases)
        {
            await VerifyCaseAsync(authorizationCase);
        }
    }

    private static async Task<AuthorizationMatrix> LoadMatrixAsync()
    {
        var fixturePath = Path.Combine(
            AppContext.BaseDirectory,
            "fixtures",
            "identity-authorization-cases.json");
        return JsonSerializer.Deserialize<AuthorizationMatrix>(
            await File.ReadAllTextAsync(fixturePath, TestContext.Current.CancellationToken),
            SerializerOptions) ?? throw new InvalidDataException("The authorization matrix is empty.");
    }

    private async Task VerifyCaseAsync(AuthorizationCase authorizationCase)
    {
        var actorId = Guid.CreateVersion7();
        var targetId = Guid.CreateVersion7();
        var otherId = Guid.CreateVersion7();

        await using (var scope = Services.CreateAsyncScope())
        {
            var dataSource = scope.ServiceProvider.GetRequiredService<NpgsqlDataSource>();
            await using var connection = await dataSource.OpenConnectionAsync(TestContext.Current.CancellationToken);
            await using var transaction = await connection.BeginTransactionAsync(TestContext.Current.CancellationToken);

            await ExecuteAsync(
                connection,
                transaction,
                "INSERT INTO actors (id, isenabled, type) VALUES (@id, @enabled, @type)",
                ("id", actorId),
                ("enabled", authorizationCase.ActorEnabled),
                ("type", authorizationCase.PrincipalType.ToString()));

            await InsertRoleGrantsAsync(
                connection,
                transaction,
                actorId,
                authorizationCase.DirectRoleGrants,
                authorizationCase.Name);
            await InsertResourceGrantsAsync(
                connection,
                transaction,
                actorId,
                authorizationCase.DirectResourceGrants,
                targetId,
                otherId);

            if (authorizationCase.TeamEnabled.HasValue
                || authorizationCase.TeamRoleGrants.Count > 0
                || authorizationCase.TeamResourceGrants.Count > 0)
            {
                var teamActorId = Guid.CreateVersion7();
                var teamId = Guid.CreateVersion7();
                await ExecuteAsync(
                    connection,
                    transaction,
                    "INSERT INTO actors (id, isenabled, type) VALUES (@id, @enabled, 'Team')",
                    ("id", teamActorId),
                    ("enabled", authorizationCase.TeamEnabled ?? true));
                await ExecuteAsync(
                    connection,
                    transaction,
                    "INSERT INTO teams (id, actorid, name) VALUES (@id, @actorId, @name)",
                    ("id", teamId),
                    ("actorId", teamActorId),
                    ("name", $"differential-team-{teamId:N}"));
                await ExecuteAsync(
                    connection,
                    transaction,
                    "INSERT INTO actorteammemberships (memberactorid, teamid) VALUES (@actorId, @teamId)",
                    ("actorId", actorId),
                    ("teamId", teamId));
                await InsertRoleGrantsAsync(
                    connection,
                    transaction,
                    teamActorId,
                    authorizationCase.TeamRoleGrants,
                    authorizationCase.Name);
                await InsertResourceGrantsAsync(
                    connection,
                    transaction,
                    teamActorId,
                    authorizationCase.TeamResourceGrants,
                    targetId,
                    otherId);
            }

            await transaction.CommitAsync(TestContext.Current.CancellationToken);
        }

        var isAdministrator = authorizationCase.PrincipalType == AuthenticatedPrincipalType.User
            && UserContextAccessor.IsAdmin([.. authorizationCase.Roles]);
        Assert.Equal(authorizationCase.ExpectedAdministrator, isAdministrator);

        await using var verificationScope = Services.CreateAsyncScope();
        var unitOfWork = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actorScope = await unitOfWork.Users.GetActorScopeAsync(
            actorId,
            TestContext.Current.CancellationToken);
        foreach (var probe in authorizationCase.Probes)
        {
            var resourceId = probe.Scope switch
            {
                ProbeScope.Global => (Guid?)null,
                ProbeScope.Target => targetId,
                ProbeScope.Other => otherId,
                _ => throw new ArgumentOutOfRangeException(nameof(probe.Scope))
            };
            var permission = await unitOfWork.Users.GetEffectivePermissionsAsync(
                actorScope,
                probe.ResourceType,
                resourceId,
                TestContext.Current.CancellationToken);
            var actual = permission.Has(probe.Level, probe.Specific ?? SpecificPermission.None);
            Assert.True(
                actual == probe.Expected,
                $"{authorizationCase.Name}: {probe.Scope} {probe.ResourceType} "
                + $"{probe.Level}/{probe.Specific?.ToString() ?? "None"}; expected {probe.Expected}, got {actual}.");
        }
    }

    private static async Task InsertRoleGrantsAsync(
        NpgsqlConnection connection,
        NpgsqlTransaction transaction,
        Guid actorId,
        IReadOnlyList<RoleGrant> grants,
        string caseName)
    {
        for (var index = 0; index < grants.Count; index++)
        {
            var grant = grants[index];
            var roleId = Guid.CreateVersion7();
            await ExecuteAsync(
                connection,
                transaction,
                "INSERT INTO roles (id, name, roletype) VALUES (@id, @name, 'Custom')",
                ("id", roleId),
                ("name", $"differential-{caseName}-{index}-{Guid.CreateVersion7():N}"));
            await ExecuteAsync(
                connection,
                transaction,
                "INSERT INTO actorroles (actorid, roleid) VALUES (@actorId, @roleId)",
                ("actorId", actorId),
                ("roleId", roleId));
            await ExecuteAsync(
                connection,
                transaction,
                "INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions) "
                + "VALUES (@id, @level, @resourceType, @roleId, @specifics)",
                ("id", Guid.CreateVersion7()),
                ("level", (int)grant.Level),
                ("resourceType", (int)grant.ResourceType),
                ("roleId", roleId),
                ("specifics", SpecificMask(grant.Specifics)));
        }
    }

    private static async Task InsertResourceGrantsAsync(
        NpgsqlConnection connection,
        NpgsqlTransaction transaction,
        Guid actorId,
        IReadOnlyList<ResourceGrant> grants,
        Guid targetId,
        Guid otherId)
    {
        foreach (var grant in grants)
        {
            var resourceId = grant.Resource switch
            {
                ResourceTarget.Target => targetId,
                ResourceTarget.Other => otherId,
                _ => throw new ArgumentOutOfRangeException(nameof(grant.Resource))
            };
            await ExecuteAsync(
                connection,
                transaction,
                "INSERT INTO resourceaccesses "
                + "(id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions) "
                + "VALUES (@id, @actorId, @level, @resourceId, @resourceType, @specifics)",
                ("id", Guid.CreateVersion7()),
                ("actorId", actorId),
                ("level", (int)grant.Level),
                ("resourceId", resourceId),
                ("resourceType", (int)grant.ResourceType),
                ("specifics", SpecificMask(grant.Specifics)));
        }
    }

    private static int SpecificMask(IEnumerable<SpecificPermission> specifics)
        => specifics.Aggregate(0, (mask, permission) => mask | (int)permission);

    private static async Task ExecuteAsync(
        NpgsqlConnection connection,
        NpgsqlTransaction transaction,
        string sql,
        params (string Name, object Value)[] parameters)
    {
        await using var command = new NpgsqlCommand(sql, connection, transaction);
        foreach (var parameter in parameters)
        {
            command.Parameters.AddWithValue(parameter.Name, parameter.Value);
        }

        await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken);
    }

    private sealed record AuthorizationMatrix(
        int SchemaVersion,
        IReadOnlyDictionary<ResourceType, ExpectedPermissionCapability> PermissionMatrix,
        IReadOnlyList<AuthorizationCase> Cases);

    private sealed record ExpectedPermissionCapability(
        PermissionLevel MaximumLevel,
        IReadOnlyDictionary<SpecificPermission, PermissionLevel> Specifics);

    private sealed record AuthorizationCase(
        string Name,
        AuthenticatedPrincipalType PrincipalType,
        IReadOnlyList<string> Roles,
        bool ActorEnabled,
        bool? TeamEnabled,
        IReadOnlyList<RoleGrant> DirectRoleGrants,
        IReadOnlyList<RoleGrant> TeamRoleGrants,
        IReadOnlyList<ResourceGrant> DirectResourceGrants,
        IReadOnlyList<ResourceGrant> TeamResourceGrants,
        bool ExpectedAdministrator,
        IReadOnlyList<AuthorizationProbe> Probes);

    private sealed record RoleGrant(
        ResourceType ResourceType,
        PermissionLevel Level,
        IReadOnlyList<SpecificPermission> Specifics);

    private sealed record ResourceGrant(
        ResourceTarget Resource,
        ResourceType ResourceType,
        PermissionLevel Level,
        IReadOnlyList<SpecificPermission> Specifics);

    private sealed record AuthorizationProbe(
        ProbeScope Scope,
        ResourceType ResourceType,
        PermissionLevel Level,
        SpecificPermission? Specific,
        bool Expected);

    private enum ResourceTarget
    {
        Target,
        Other
    }

    private enum ProbeScope
    {
        Global,
        Target,
        Other
    }
}
