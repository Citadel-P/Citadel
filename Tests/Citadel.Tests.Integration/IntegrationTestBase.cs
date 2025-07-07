using System.Data;
using System.Net.Http.Headers;
using System.Security.Claims;
using Application.Services;
using DbUp;
using Domain.Contracts.Interfaces;
using Infrastructure;
using Infrastructure.Persistence;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration;

public abstract class IntegrationTestBase : IAsyncLifetime
{
    private SqliteConnection keepAliveConnection = default!;
    private WebApplicationFactory<WebApi.Program> _factory = default!;
    private readonly string connectionString = $"Data Source={Guid.NewGuid()};Mode=Memory;Cache=Shared";

    protected HttpClient Client = default!;
    protected IServiceProvider Services = default!;

    public async ValueTask InitializeAsync()
    {
        keepAliveConnection = new SqliteConnection(connectionString);
        await keepAliveConnection.OpenAsync();

        RunMigrations(keepAliveConnection);

        _factory = new WebApplicationFactory<WebApi.Program>()
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

        Client = _factory.CreateClient();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());
    }

    protected virtual void ConfigureTestServices(IServiceCollection services) { }

    protected virtual ValueTask SeedDbAsync(IUnitOfWork uow) => ValueTask.CompletedTask;

    protected string CreateJwtToken(IEnumerable<Claim>? claims = null)
    {
        var jwt = Services.GetRequiredService<IJwtService>();
        return jwt.CreateAccessToken(claims ?? new[]
        {
            new Claim("role", "admin"),
            new Claim("name", "Test user")
        });
    }

    private void ReplaceTestServices(IServiceCollection services)
    {
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
        await _factory.DisposeAsync();
        await keepAliveConnection.DisposeAsync();
    }
}
