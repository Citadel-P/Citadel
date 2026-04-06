using Application.Services;
using DbUp;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Infrastructure;
using Infrastructure.Persistence;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.DependencyInjection;
using System.Data;
using System.Data.Common;
using System.Net.Http.Headers;
using System.Security.Claims;
using Tests.Integration.Helpers;

namespace Tests.Integration;

public abstract class IntegrationTestBase : IAsyncLifetime
{
    protected sealed record AuthorizationSubject(Guid UserId, Guid ActorId, Guid? TeamId = null, Guid? TeamActorId = null);
    protected sealed record ResourceGrant(ResourceType ResourceType, Guid ResourceId, ResourceAction Action);

    protected static readonly Guid OperatorRoleId = Guid.Parse("30000000-0000-0000-0000-000000000002");
    protected static readonly Guid ViewerRoleId = Guid.Parse("30000000-0000-0000-0000-000000000003");

    private SqliteConnection keepAliveConnection = default!;
    private WebApplicationFactory<Program> factory = default!;
    private readonly string connectionString = $"Data Source={Guid.NewGuid()};Mode=Memory;Cache=Shared";

    protected HttpClient Client = default!;
    protected IServiceProvider Services = default!;

    public async ValueTask InitializeAsync()
    {
        keepAliveConnection = new SqliteConnection(connectionString);
        await keepAliveConnection.OpenAsync();

        RunMigrations(keepAliveConnection);

        factory = new WebApplicationFactory<Program>()
            .WithWebHostBuilder(builder =>
            {
                builder.UseEnvironment("IntegrationTests");

                builder.ConfigureServices(async services =>
                {
                    ReplaceTestServices(services);
                    ConfigureTestServices(services);

                    var sp = services.BuildServiceProvider();
                    Services = sp;

                    await using var scope = sp.CreateAsyncScope();
                    await SeedDbAsync(scope.ServiceProvider.GetRequiredService<IUnitOfWork>());
                });
            });

        Client = factory.CreateClient();
        Services = factory.Services;
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());
    }

    protected HttpMessageHandler CreateServerHandler() => factory.Server.CreateHandler();

    protected virtual void ConfigureTestServices(IServiceCollection services) { }

    protected virtual ValueTask SeedDbAsync(IUnitOfWork uow) => ValueTask.CompletedTask;

    protected string CreateJwtToken(IEnumerable<Claim>? claims = null)
    {
        var jwt = Services.GetRequiredService<IJwtService>();
        return jwt.CreateAccessToken(claims ?? new[] {
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
            new("sub", userId.ToString()),
            new("name", "Test user"),
            new("actorId", actorId.ToString())
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
        await using var transaction = await connection.BeginTransactionAsync(TestContext.Current.CancellationToken);

        await ExecuteNonQueryAsync(connection, transaction,
            "INSERT INTO Actors (Id, Type) VALUES (@Id, @Type);",
            new Dictionary<string, object?>
            {
                ["@Id"] = actorId.ToString(),
                ["@Type"] = "User"
            });

        await ExecuteNonQueryAsync(connection, transaction,
            "INSERT INTO Users (Id, ActorId, Name, Email, Password, CreatedAt, CreatedByActorId) VALUES (@Id, @ActorId, @Name, @Email, @Password, @CreatedAt, @CreatedByActorId);",
            new Dictionary<string, object?>
            {
                ["@Id"] = userId.ToString(),
                ["@ActorId"] = actorId.ToString(),
                ["@Name"] = "Test user",
                ["@Email"] = email,
                ["@Password"] = null,
                ["@CreatedAt"] = now,
                ["@CreatedByActorId"] = Constants.SystemId.ToString()
            });

        if (directRoleId.HasValue)
        {
            await ExecuteNonQueryAsync(connection, transaction,
                "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId);",
                new Dictionary<string, object?>
                {
                    ["@ActorId"] = actorId.ToString(),
                    ["@RoleId"] = directRoleId.Value.ToString()
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
                    ["@Id"] = teamActorId.Value.ToString(),
                    ["@Type"] = "Team"
                });

            await ExecuteNonQueryAsync(connection, transaction,
                "INSERT INTO Teams (Id, ActorId, Name) VALUES (@Id, @ActorId, @Name);",
                new Dictionary<string, object?>
                {
                    ["@Id"] = teamId.Value.ToString(),
                    ["@ActorId"] = teamActorId.Value.ToString(),
                    ["@Name"] = $"team-{teamId.Value:N}"
                });

            await ExecuteNonQueryAsync(connection, transaction,
                "INSERT INTO UsersTeams (UserId, TeamId) VALUES (@UserId, @TeamId);",
                new Dictionary<string, object?>
                {
                    ["@UserId"] = userId.ToString(),
                    ["@TeamId"] = teamId.Value.ToString()
                });

            if (teamRoleId.HasValue)
            {
                await ExecuteNonQueryAsync(connection, transaction,
                    "INSERT INTO ActorRoles (ActorId, RoleId) VALUES (@ActorId, @RoleId);",
                    new Dictionary<string, object?>
                    {
                        ["@ActorId"] = teamActorId.Value.ToString(),
                        ["@RoleId"] = teamRoleId.Value.ToString()
                    });
            }
        }

        if (resourceGrants is not null)
        {
            foreach (var grant in resourceGrants)
            {
                await ExecuteNonQueryAsync(connection, transaction,
                    "INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, Action) VALUES (@Id, @ResourceId, @ActorId, @ResourceType, @Action);",
                    new Dictionary<string, object?>
                    {
                        ["@Id"] = Guid.CreateVersion7().ToString(),
                        ["@ResourceId"] = grant.ResourceId.ToString(),
                        ["@ActorId"] = actorId.ToString(),
                        ["@ResourceType"] = grant.ResourceType.ToString(),
                        ["@Action"] = grant.Action.ToString()
                    });
            }
        }

        await transaction.CommitAsync(TestContext.Current.CancellationToken);

        return new AuthorizationSubject(userId, actorId, teamId, teamActorId);
    }

    private static async Task ExecuteNonQueryAsync(
        SqliteConnection connection,
        DbTransaction transaction,
        string commandText,
        IReadOnlyDictionary<string, object?> parameters)
    {
        await using var command = connection.CreateCommand();
        command.Transaction = (SqliteTransaction)transaction;
        command.CommandText = commandText;

        foreach (var parameter in parameters)
        {
            command.Parameters.AddWithValue(parameter.Key, parameter.Value ?? DBNull.Value);
        }

        await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken);
    }

    private void ReplaceTestServices(IServiceCollection services)
    {
        services.ReplaceService<IPlatformContainerCache>(new PlatformContainerCache());
        services.ReplaceService<IDbConnectionFactory>(new SqliteConnectionFactory(connectionString));
    }

    private static void RunMigrations(IDbConnection dbConnection)
    {
        var upgrader = DeployChanges.To
            .SqliteDatabase(dbConnection.ConnectionString)
            .WithScriptsAndCodeEmbeddedInAssembly(typeof(InfrastructureModule).Assembly)
            .LogToConsole()
            .Build();

        var result = upgrader.PerformUpgrade();
        if (!result.Successful)
        {
            throw new Exception($"Failed to upgrade in-memory test database: {result.Error.Message}", result.Error);
        }
    }

    public async ValueTask DisposeAsync()
    {
        GC.SuppressFinalize(this);
        await factory.DisposeAsync();
        await keepAliveConnection.DisposeAsync();
    }
}