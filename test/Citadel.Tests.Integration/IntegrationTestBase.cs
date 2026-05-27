using Application.Services;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Infrastructure.Persistence;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Npgsql;
using System.Data.Common;
using System.Net.Http.Headers;
using System.Security.Claims;
using System.IdentityModel.Tokens.Jwt;
using Tests.Integration.Helpers;

namespace Tests.Integration;


[Collection("Postgres")]
public abstract class IntegrationTestBase(PostgresTestFixture fixture) : IAsyncLifetime
{
    protected sealed record AuthorizationSubject(Guid UserId, Guid ActorId, Guid? TeamId = null, Guid? TeamActorId = null);
    protected sealed record ResourceGrant(ResourceType ResourceType, Guid ResourceId, PermissionLevel PermissionLevel, SpecificPermission SpecificPermission = SpecificPermission.None);

    private sealed class TestDbConnectionFactory(NpgsqlDataSource dataSource) : IDbConnectionFactory
    {
        public DbConnection Create() => dataSource.CreateConnection();
    }

    protected static readonly Guid OperatorRoleId = Guid.Parse("30000000-0000-0000-0000-000000000002");
    protected static readonly Guid ViewerRoleId = Guid.Parse("30000000-0000-0000-0000-000000000003");

    protected HttpClient Client = default!;
    protected string ConnectionString = default!;
    protected IServiceProvider Services = default!;
    protected WebApplicationFactory<Program> Factory = default!;

    public async ValueTask InitializeAsync()
    {
        ConnectionString = await fixture.CreateDatabaseFromTemplateAsync();

        Factory = new WebApplicationFactory<Program>()
            .WithWebHostBuilder(builder =>
            {
                builder.UseEnvironment("IntegrationTests");

                builder.ConfigureAppConfiguration((_, config) =>
                {
                    config.AddInMemoryCollection(new Dictionary<string, string?>
                    {
                        ["ConnectionStrings:Postgres"] = ConnectionString
                    });
                });

                builder.ConfigureServices(services =>
                {
                    services.RemoveAll<IHostedService>();
                    services.ReplaceService<NpgsqlDataSource>(_ =>
                    {
                        var builder = new NpgsqlDataSourceBuilder(ConnectionString);
                        return builder.Build();
                    });
                    services.ReplaceService<IDbConnectionFactory>(sp =>
                        new TestDbConnectionFactory(sp.GetRequiredService<NpgsqlDataSource>()));
                    services.ReplaceService<IPlatformContainerCache>(new PlatformContainerCache());
                    ConfigureTestServices(services);
                });
            });

        Services = Factory.Services;
        Client = Factory.CreateClient();
        await SeedDbAsync(Factory.Services.GetRequiredService<IUnitOfWork>());
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());
    }

    protected HttpMessageHandler CreateServerHandler() => Factory.Server.CreateHandler();

    protected virtual void ConfigureTestServices(IServiceCollection services) { }

    protected virtual ValueTask SeedDbAsync(IUnitOfWork uow) => ValueTask.CompletedTask;

    protected string CreateJwtToken(IEnumerable<Claim>? claims = null)
    {
        var jwt = Services.GetRequiredService<IJwtService>();
        return jwt.CreateAccessToken(claims ?? new[] {
            new Claim(JwtRegisteredClaimNames.Sub, Constants.DefaultAdminId.ToString()),
            new Claim("role", "admin"),
            new Claim("name", "Test user"),
            new Claim("actorId", Constants.SystemId.ToString())
        });
    }

    protected string CreateJwtToken(Guid userId, Guid actorId, IEnumerable<Claim>? claims = null)
    {
        var jwt = Services.GetRequiredService<IJwtService>();
        var allClaims = new List<Claim>
        {
            new Claim(JwtRegisteredClaimNames.Sub, userId.ToString()),
            new Claim("name", "Test user"),
            new Claim("actorId", actorId.ToString())
        };

        if (claims is not null)
        {
            allClaims.AddRange(claims);
        }

        return jwt.CreateAccessToken(allClaims);
    }

    protected async Task<AuthorizationSubject> CreateAuthorizationSubjectAsync(
        Guid? directRoleId = null,
        Guid? teamRoleId = null,
        bool createTeam = false,
        IEnumerable<ResourceGrant>? resourceGrants = null)
    {
        var userId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var email = $"{userId:N}@citadel.test";
        var now = DateTime.UtcNow;

        Guid? teamId = null;
        Guid? teamActorId = null;

        await using var scope = Services.CreateAsyncScope();
        var connectionFactory = scope.ServiceProvider.GetRequiredService<IDbConnectionFactory>();
        await using var connection = connectionFactory.Create();
        await connection.OpenAsync(TestContext.Current.CancellationToken);
        await using var transaction = await connection.BeginTransactionAsync(TestContext.Current.CancellationToken);

        await ExecuteNonQueryAsync(connection, transaction,
            "INSERT INTO Actors (Id, Type) VALUES (@Id, @Type);",
            new Dictionary<string, object?>
            {
                ["@Id"] = actorId,
                ["@Type"] = "User"
            });

        await ExecuteNonQueryAsync(connection, transaction,
            "INSERT INTO Users (Id, ActorId, Name, Email, Password, CreatedAt, CreatedByActorId) VALUES (@Id, @ActorId, @Name, @Email, @Password, @CreatedAt, @CreatedByActorId);",
            new Dictionary<string, object?>
            {
                ["@Id"] = userId,
                ["@ActorId"] = actorId,
                ["@Name"] = "Test user",
                ["@Email"] = email,
                ["@Password"] = null,
                ["@CreatedAt"] = now,
                ["@CreatedByActorId"] = Constants.SystemId
            });

        if (directRoleId.HasValue)
        {
            await ExecuteNonQueryAsync(connection, transaction,
                "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId);",
                new Dictionary<string, object?>
                {
                    ["@ActorId"] = actorId,
                    ["@RoleId"] = directRoleId.Value
                });
        }

        if (createTeam || teamRoleId.HasValue)
        {
            teamId = Guid.CreateVersion7();
            teamActorId = Guid.CreateVersion7();

            await ExecuteNonQueryAsync(connection, transaction,
                "INSERT INTO Actors (Id, Type) VALUES (@Id, @Type);",
                new Dictionary<string, object?>
                {
                    ["@Id"] = teamActorId.Value,
                    ["@Type"] = "Team"
                });

            await ExecuteNonQueryAsync(connection, transaction,
                "INSERT INTO Teams (Id, ActorId, Name) VALUES (@Id, @ActorId, @Name);",
                new Dictionary<string, object?>
                {
                    ["@Id"] = teamId.Value,
                    ["@ActorId"] = teamActorId.Value,
                    ["@Name"] = $"team-{teamId.Value:N}"
                });

            await ExecuteNonQueryAsync(connection, transaction,
                "INSERT INTO UsersTeams (UserId, TeamId) VALUES (@UserId, @TeamId);",
                new Dictionary<string, object?>
                {
                    ["@UserId"] = userId,
                    ["@TeamId"] = teamId.Value
                });

            if (teamRoleId.HasValue)
            {
                await ExecuteNonQueryAsync(connection, transaction,
                    "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId);",
                    new Dictionary<string, object?>
                    {
                        ["@ActorId"] = teamActorId.Value,
                        ["@RoleId"] = teamRoleId.Value
                    });
            }
        }

        if (resourceGrants is not null)
        {
            foreach (var grant in resourceGrants)
            {
                await ExecuteNonQueryAsync(connection, transaction,
                    "INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, PermissionLevel, SpecificPermissions) VALUES (@Id, @ResourceId, @ActorId, @ResourceType, @PermissionLevel, @SpecificPermissions);",
                    new Dictionary<string, object?>
                    {
                        ["@Id"] = Guid.CreateVersion7(),
                        ["@ResourceId"] = grant.ResourceId,
                        ["@ActorId"] = actorId,
                        ["@ResourceType"] = (int)grant.ResourceType,
                        ["@PermissionLevel"] = (int)grant.PermissionLevel,
                        ["@SpecificPermissions"] = Domain.Entities.Identity.Permission.ToSpecificPermissionsMask(
                            grant.SpecificPermission == SpecificPermission.None
                                ? Array.Empty<SpecificPermission>()
                                : new[] { grant.SpecificPermission })
                    });
            }
        }

        await transaction.CommitAsync(TestContext.Current.CancellationToken);

        return new AuthorizationSubject(userId, actorId, teamId, teamActorId);
    }

    private static async Task ExecuteNonQueryAsync(
        DbConnection connection,
        DbTransaction transaction,
        string commandText,
        IReadOnlyDictionary<string, object?> parameters)
    {
        await using var command = connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText = commandText;

        foreach (var parameter in parameters)
        {
            var dbParameter = command.CreateParameter();
            dbParameter.ParameterName = parameter.Key;
            dbParameter.Value = parameter.Value ?? DBNull.Value;
            command.Parameters.Add(dbParameter);
        }

        await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken);
    }

    protected async Task SetActorEnabledAsync(Guid actorId, bool isEnabled)
    {
        await using var scope = Services.CreateAsyncScope();
        var connectionFactory = scope.ServiceProvider.GetRequiredService<IDbConnectionFactory>();
        await using var connection = connectionFactory.Create();
        await connection.OpenAsync(TestContext.Current.CancellationToken);
        await using var transaction = await connection.BeginTransactionAsync(TestContext.Current.CancellationToken);

        await ExecuteNonQueryAsync(
            connection,
            transaction,
            "UPDATE Actors SET IsEnabled = @IsEnabled WHERE Id = @Id;",
            new Dictionary<string, object?>
            {
                ["@Id"] = actorId,
                ["@IsEnabled"] = isEnabled
            });

        await transaction.CommitAsync(TestContext.Current.CancellationToken);
    }

    public async ValueTask DisposeAsync()
    {
        Client?.Dispose();
        if (Factory != null) await Factory.DisposeAsync();

        NpgsqlConnection.ClearAllPools();
    }
}