using System.Net.Http.Headers;
using System.Security.Claims;
using Application.Services;
using Citadel.Agent.Common.V1;
using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.EntityFramework;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration;

public abstract class IntegrationTestBase<TEntryPoint> : IAsyncLifetime
    where TEntryPoint : class
{
    private SqliteConnection connection = default!;
    private WebApplicationFactory<TEntryPoint> factory = default!;
    protected HttpClient Client = default!;
    protected IServiceProvider Services = default!;

    public async ValueTask InitializeAsync()
    {
        connection = new SqliteConnection("DataSource=:memory:");
        await connection.OpenAsync();

        factory = new WebApplicationFactory<TEntryPoint>()
            .WithWebHostBuilder(builder =>
            {
                builder.ConfigureServices(services =>
                {
                    // Remove existing DbContextOptions<AppDbContext>
                    var descriptor = services.SingleOrDefault(
                        d => d.ServiceType == typeof(DbContextOptions<ApplicationDbContext>));
                    if (descriptor != null) services.Remove(descriptor);

                    services.AddDbContextPool<ApplicationDbContext>(options =>
                    {
                        options.UseSqlite(connection);
                    });

                    ConfigureTestServices(services);

                    // Build the provider and seed database
                    var sp = services.BuildServiceProvider();
                    using var scope = sp.CreateScope();
                    var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
                    db.Database.EnsureCreated();

                    Services = sp;
                    SeedDbAsync().GetAwaiter().GetResult();
                });
            });

        
        Client = factory.CreateClient();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtTokenAsync());
    }

    /// <summary>
    /// Override this method to configure additional services for testing
    /// </summary>
    protected virtual void ConfigureTestServices(IServiceCollection services) { }

    /// <summary>
    /// Override to seed the database with initial data
    /// </summary>
    protected virtual ValueTask SeedDbAsync() => ValueTask.CompletedTask;

    /// <summary>
    /// Override to create new roles, users, etc
    /// </summary>
    protected string CreateJwtTokenAsync(IEnumerable<Claim>? claims = null)
    {
        // Create a jwt token
        var jwt = Services.GetRequiredService<IJwtService>();
        var token = jwt.CreateAccessToken(claims ??
        [
            new ("role", "admin"),
            new ("name", "Test user"),
        ]);
        return token;
    }

    public async ValueTask DisposeAsync()
    {
        await connection.DisposeAsync();
        factory.Dispose();
    }

    protected PlatformResult GetDummyPlatformResult() =>
        new
        (
            Name: "p-01",
            Address: "https://original.address",
            NetworkCount: 1,
            VolumeCount: 2,
            ImageCount: 3,
            CpuCount: 4,
            MemTotal: 500,
            ServerVersion: "1.0.0",
            AgentVersion: "1.0.0",
            Descriptor: new DockerPlatformDescriptor
                (
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
        var platformDescriptor = new DockerPlatformDescriptor(
           DaemonId: "123456",
           ContainerCount: 5,
           ContainersRunning: 2,
           ContainersPaused: 2,
           ContainersStopped: 1);

        var platform = new Platform(
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
            platformDescriptor: platformDescriptor
        );
        return platform;
    }

    protected IEnumerable<DockerContainer> GetDummyContainers(int total = 3)
    {
        for (int i = 0; i < total; i++)
        {
            yield return new
            (
                Name: $"c-{i:D2}",
                Image: $"image-{i}:latest",
                State: ContainerStateStatus.Running,
                ContainerId: "container" + i
            );
        }

    }
}