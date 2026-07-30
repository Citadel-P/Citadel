using System.Collections.Concurrent;
using Application.Services;
using Application.Services.Identity;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
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
using System.Reflection;
using System.Security.Claims;
using System.IdentityModel.Tokens.Jwt;
using Tests.Integration.Helpers;
using Tests.Common;

namespace Tests.Integration;


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
    private static readonly Guid SeededAdminUserId = Guid.Parse("10000000-0000-0000-0000-000000000001");
    private static readonly Guid AdminRoleId = Guid.Parse("30000000-0000-0000-0000-000000000001");
    private static readonly ConcurrentDictionary<string, string> TestPasswordHashes =
        new(StringComparer.Ordinal);

    protected HttpClient Client = default!;
    protected string ConnectionString = default!;
    protected IServiceProvider Services = default!;
    protected WebApplicationFactory<Program> Factory = default!;
    private ReusableIntegrationTestHost? reusableHost;

    public async ValueTask InitializeAsync()
    {
        if (ReuseApplicationFactory)
        {
            reusableHost = await fixture.GetOrCreateReusableHostAsync(
                GetType(),
                CreateFactory);
            await reusableHost.PrepareForTestAsync(fixture);

            ConnectionString = reusableHost.ConnectionString;
            Factory = reusableHost.Factory;
            Services = reusableHost.Services;
        }
        else
        {
            ConnectionString = await fixture.CreateDatabaseFromTemplateAsync();
            Factory = CreateFactory(ConnectionString);
            Services = Factory.Services;
        }

        Client = Factory.CreateClient();
        await using (var scope = Services.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            if (SeedDefaultAdministrator)
            {
                await SeedDefaultAdministratorAsync(scope.ServiceProvider, unitOfWork);
            }

            await SeedDbAsync(unitOfWork);
        }
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());
    }

    private WebApplicationFactory<Program> CreateFactory(string connectionString)
        => new WebApplicationFactory<Program>()
            .WithWebHostBuilder(builder =>
            {
                builder.UseEnvironment("IntegrationTests");

                builder.ConfigureAppConfiguration((_, config) =>
                {
                    config.AddInMemoryCollection(new Dictionary<string, string?>
                    {
                        ["ConnectionStrings:Postgres"] = connectionString,
                        ["Secrets:EncryptionKey"] = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=",
                        ["Transport:Mode"] = "Disabled",
                        ["Transport:PublicUrl"] = "http://localhost:8000",
                        ["EdgeAgent:PublicGrpcUrl"] = "http://localhost:8001",
                        ["AgentTransport:AllowInsecure"] = "true"
                    });
                });

                builder.ConfigureServices(services =>
                {
                    services.RemoveAll<IHostedService>();
                    services.ReplaceService<NpgsqlDataSource>(_ =>
                    {
                        var builder = new NpgsqlDataSourceBuilder(connectionString);
                        return builder.Build();
                    });
                    services.ReplaceService<IDbConnectionFactory>(sp =>
                        new TestDbConnectionFactory(sp.GetRequiredService<NpgsqlDataSource>()));
                    services.ReplaceService<IPlatformContainerCache>(new PlatformContainerCache());
                    if (!UseRealLicenseEntitlements)
                    {
                        services.ReplaceService<ILicenseEntitlementService>(
                            new PermissiveLicenseEntitlementService());
                    }
                    ConfigureTestServices(services);
                });
            });

    protected HttpMessageHandler CreateServerHandler() => Factory.Server.CreateHandler();

    // Service override delegates commonly capture mocks owned by one xUnit test instance.
    protected virtual bool ReuseApplicationFactory
        => GetType()
            .GetMethod(
                nameof(ConfigureTestServices),
                BindingFlags.Instance | BindingFlags.NonPublic)!
            .DeclaringType == typeof(IntegrationTestBase);

    protected virtual bool UseRealLicenseEntitlements => false;
    protected virtual bool SeedDefaultAdministrator => true;

    protected virtual void ConfigureTestServices(IServiceCollection services) { }

    protected virtual ValueTask SeedDbAsync(IUnitOfWork uow) => ValueTask.CompletedTask;

    private static async Task SeedDefaultAdministratorAsync(
        IServiceProvider services,
        IUnitOfWork unitOfWork)
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var setupState = await unitOfWork.InstanceSetupState.GetLockedAsync(cancellationToken)
            ?? throw new InvalidOperationException("The integration database has no setup state.");

        if (!setupState.RequiresSetup)
        {
            services.GetRequiredService<ISetupStateCache>().SetRequiresSetup(false);
            return;
        }

        var actor = Actor.FromPersistence(
            Constants.DefaultAdminId,
            ActorType.User,
            new ActorMetadata("admin"),
            isEnabled: true);
        var user = User.FromPersistence(
            SeededAdminUserId,
            "admin",
            "admin@citadel.local",
            HashTestPassword("admin123"),
            actor.Id,
            Constants.SystemId,
            DateTime.UtcNow);

        setupState.TryComplete(actor.Id, DateTimeOffset.UtcNow);
        await unitOfWork.Actors.AddAsync(actor, cancellationToken);
        await unitOfWork.Users.AddAsync(user, cancellationToken);
        await unitOfWork.Roles.AddActorRoleAsync(actor.Id, AdminRoleId, cancellationToken);
        await unitOfWork.InstanceSetupState.UpdateAsync(setupState, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        services.GetRequiredService<ISetupStateCache>().SetRequiresSetup(false);
    }

    protected static string HashTestPassword(string password)
        => TestPasswordHashes.GetOrAdd(
            password,
            static value => new CitadelPasswordHasher().Hash(value));

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
                ["@Password"] = string.Empty,
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

    public virtual async ValueTask DisposeAsync()
    {
        Client?.Dispose();
        if (reusableHost is not null)
        {
            return;
        }

        try
        {
            if (Factory != null)
            {
                await Factory.DisposeAsync();
            }
        }
        finally
        {
            if (!string.IsNullOrWhiteSpace(ConnectionString))
            {
                await fixture.DropDatabaseAsync(ConnectionString);
            }
        }
    }
}
