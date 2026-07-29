using Application.Services.Alerts;
using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.DependencyInjection;
using Npgsql;

namespace Tests.Integration;

internal sealed class ReusableIntegrationTestHost(
    string connectionString,
    WebApplicationFactory<Program> factory) : IAsyncDisposable
{
    private readonly SemaphoreSlim prepareLock = new(1, 1);
    private bool hasPreparedTest;

    public string ConnectionString { get; } = connectionString;
    public WebApplicationFactory<Program> Factory { get; } = factory;
    public IServiceProvider Services => Factory.Services;

    public async Task PrepareForTestAsync(PostgresTestFixture postgres)
    {
        await prepareLock.WaitAsync();
        try
        {
            if (!hasPreparedTest)
            {
                hasPreparedTest = true;
                return;
            }

            Services.GetRequiredService<NpgsqlDataSource>().Clear();
            await postgres.ResetDatabaseFromTemplateAsync(ConnectionString);
            await ResetInMemoryStateAsync();
        }
        finally
        {
            prepareLock.Release();
        }
    }

    private async Task ResetInMemoryStateAsync()
    {
        Services.GetRequiredService<ISetupStateCache>().Reset();

        if (Services.GetService<IMemoryCache>() is MemoryCache memoryCache)
        {
            memoryCache.Compact(1);
        }

        var platformCache = Services.GetRequiredService<IPlatformContainerCache>();
        if (platformCache.TryGetCacheEntries(out var entries, out _))
        {
            foreach (var entry in entries.ToArray())
            {
                platformCache.EvictPlatform(entry.Id);
            }
        }

        var alertRuleCache = Services.GetRequiredService<AlertRuleCache>();
        if (alertRuleCache.Current.ByType.Count > 0
            || alertRuleCache.Current.Channels.Count > 0)
        {
            await alertRuleCache.ReloadAsync();
        }
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            await Factory.DisposeAsync();
        }
        finally
        {
            prepareLock.Dispose();
        }
    }
}
