using System.Data;
using System.Net.Http.Headers;
using System.Reflection;
using System.Security.Claims;
using Application.Services;
using DbUp;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.Persistence;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration;

public abstract class IntegrationTestBase<TEntryPoint> : IAsyncLifetime
    where TEntryPoint : class
{
    private WebApplicationFactory<TEntryPoint> factory = default!;
    protected HttpClient Client = default!;
    protected IServiceProvider Services = default!;
        private string _uniqueDbName = Guid.NewGuid().ToString();

    public async ValueTask InitializeAsync()
    {
        factory = new WebApplicationFactory<TEntryPoint>()
            .WithWebHostBuilder(builder =>
            {
                builder.UseEnvironment("IntegrationTests");

                builder.ConfigureServices(async (services) =>
                {
                    RemoveService<IDbConnectionFactory>(services);
                    RemoveService<IUnitOfWork>(services);

                    services.AddSingleton<IDbConnectionFactory>(_ => new InMemoryTestDbConnectionFactory(_uniqueDbName));
                    services.AddScoped<IUnitOfWork, UnitOfWork>();

                    ConfigureTestServices(services);

                    var sp = services.BuildServiceProvider();
                    Services = sp;

                    await using var scope = sp.CreateAsyncScope();
                    var dbFactory = scope.ServiceProvider.GetRequiredService<IDbConnectionFactory>();
                    var uniqueConnectionString = ((InMemoryTestDbConnectionFactory)dbFactory).GetConnectionString();

                    await InitDbAsync(uniqueConnectionString);
                    await SeedDbAsync(scope.ServiceProvider.GetRequiredService<IUnitOfWork>());
                });
            });

        Client = factory.CreateClient();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtTokenAsync());
    }

    protected virtual void ConfigureTestServices(IServiceCollection services) { }

    private async Task InitDbAsync(string connectionString)
    {
        await using var dbUpConnection = new SqliteConnection(connectionString);
        await dbUpConnection.OpenAsync();

        var upgrader = DeployChanges.To
            .SqliteDatabase(dbUpConnection.ConnectionString)
            .WithScriptsAndCodeEmbeddedInAssembly(Assembly.Load("Citadel.Infrastructure"))
            .LogToConsole()
            .Build();

        var result = upgrader.PerformUpgrade();
        if (!result.Successful)
        {
            throw new Exception($"Failed to upgrade in-memory test database: {result.Error.Message}", result.Error);
        }
    }

    protected virtual ValueTask SeedDbAsync(IUnitOfWork uow) => ValueTask.CompletedTask;

    protected string CreateJwtTokenAsync(IEnumerable<Claim>? claims = null)
    {
        var jwt = Services.GetRequiredService<IJwtService>();
        var token = jwt.CreateAccessToken(claims ?? new[]
        {
            new Claim("role", "admin"),
            new Claim("name", "Test user"),
        });
        return token;
    }

    public async ValueTask DisposeAsync()
    {
        factory.Dispose();
    }

    protected PlatformResult GetDummyPlatformResult() => new(
        Name: "p-01",
        Address: "https://original.address",
        NetworkCount: 1,
        VolumeCount: 2,
        ImageCount: 3,
        CpuCount: 4,
        MemTotal: 500,
        ServerVersion: "1.0.0",
        AgentVersion: "1.0.0",
        Descriptor: new DockerPlatformDescriptor(
            DaemonId: "123456",
            ContainerCount: 5,
            ContainersRunning: 2,
            ContainersPaused: 3,
            ContainersStopped: 0,
            Driver: "overlay2",
            OperatingSystem: "Linux",
            OsVersion: "5.15",
            OsType: "linux",
            Architecture: "x86_64"
        )
    );

    protected Platform GetDummyPlatform()
    {
        var descriptor = new DockerPlatformDescriptor(
            DaemonId: "123456",
            ContainerCount: 5,
            ContainersRunning: 2,
            ContainersPaused: 2,
            ContainersStopped: 1);

        return new Platform(
            name: "Docker-P-01",
            address: "https://original.address",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: descriptor
        );
    }

    protected IEnumerable<DockerContainer> GetDummyContainers(int total = 3)
    {
        for (int i = 0; i < total; i++)
        {
            yield return new(
                Name: $"c-{i:D2}",
                Image: $"image-{i}:latest",
                State: ContainerStateStatus.Running,
                ContainerId: $"container{i}"
            );
        }
    }

    private static void RemoveService<T>(IServiceCollection services)
    {
        var descriptor = services.FirstOrDefault(d => d.ServiceType == typeof(T));
        if (descriptor != null)
            services.Remove(descriptor);
    }

}

internal sealed class InMemoryTestDbConnectionFactory : IDbConnectionFactory, IDisposable
{
    private readonly string _connectionString;
    private readonly SqliteConnection _initialConnection;

    public InMemoryTestDbConnectionFactory(string dbName)
    {
        _connectionString = $"Data Source={dbName};Mode=Memory;Cache=Shared";

        _initialConnection = new SqliteConnection(_connectionString);
        _initialConnection.Open();

        using var cmd = _initialConnection.CreateCommand();
        cmd.CommandText = """
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA busy_timeout=3000;
        """;
        cmd.ExecuteNonQuery();
    }

    public string GetConnectionString() => _connectionString;

    public IDbConnection Create()
    {
        var conn = new SqliteConnection(_connectionString);
        conn.Open();
        return conn;
    }

    public void Dispose()
    {
        _initialConnection.Dispose();
    }
}