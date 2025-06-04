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
    private SqliteConnection _connection = default!;
    private WebApplicationFactory<TEntryPoint> _factory = default!;
    protected HttpClient Client = default!;
    protected IServiceProvider Services = default!;

    public async ValueTask InitializeAsync()
    {
        var jwtToken = string.Empty;
        _connection = new SqliteConnection("DataSource=:memory:");
        await _connection.OpenAsync();

        _factory = new WebApplicationFactory<TEntryPoint>()
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
                        options.UseSqlite(_connection);
                    });

                    // Build the provider and seed database
                    var sp = services.BuildServiceProvider();
                    using var scope = sp.CreateScope();
                    var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
                    db.Database.EnsureCreated();

                    Services = scope.ServiceProvider;
                    // Allow test classes to seed data if needed
                    SeedDb().GetAwaiter().GetResult();
                    jwtToken = CreateJwtToken().GetAwaiter().GetResult();
                });
            });

        Client = _factory.CreateClient();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", jwtToken);
    }

    /// <summary>
    /// Override to seed the database with initial data
    /// </summary>
    public virtual ValueTask SeedDb() => ValueTask.CompletedTask;

    /// <summary>
    /// Override to create new roles, users, etc
    /// </summary>
    public virtual ValueTask<string> CreateJwtToken()
    {
        // Create a jwt token
        var jwt = Services.GetRequiredService<IJwtService>();
        var token = jwt.CreateAccessToken(new List<Claim>() 
        { 
            new ("role", "admin"),
            new ("name", "Test user"),
        });
        return ValueTask.FromResult(token);
    }

    public async ValueTask DisposeAsync()
    {
        await _connection.DisposeAsync();
        _factory.Dispose();
    }
}