using System.Net.Http.Headers;
using System.Security.Claims;
using Application.Services;
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
}