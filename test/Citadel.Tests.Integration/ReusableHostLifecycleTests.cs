using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.DependencyInjection;
using Npgsql;

namespace Tests.Integration;

public sealed class ReusableHostLifecycleTests(PostgresTestFixture fixture)
    : IntegrationTestBase(fixture)
{
    private const string CacheKey = "integration-test-reusable-host-marker";
    private static WebApplicationFactory<Program>? observedFactory;

    [Fact]
    public Task ReusableHost_FirstMethod_ShouldStartFromCleanState()
        => VerifyReusableHostAndCleanStateAsync();

    [Fact]
    public Task ReusableHost_SecondMethod_ShouldStartFromCleanState()
        => VerifyReusableHostAndCleanStateAsync();

    private async Task VerifyReusableHostAndCleanStateAsync()
    {
        var previousFactory = Interlocked.CompareExchange(
            ref observedFactory,
            Factory,
            comparand: null);
        if (previousFactory is not null)
        {
            Assert.Same(previousFactory, Factory);
        }

        var memoryCache = Services.GetRequiredService<IMemoryCache>();
        Assert.False(memoryCache.TryGetValue(CacheKey, out _));
        memoryCache.Set(CacheKey, true);

        await using var connection = await Services
            .GetRequiredService<NpgsqlDataSource>()
            .OpenConnectionAsync(TestContext.Current.CancellationToken);
        await using var existsCommand = new NpgsqlCommand(
            "SELECT to_regclass('public.reusable_host_marker') IS NOT NULL",
            connection);
        Assert.False((bool)(await existsCommand.ExecuteScalarAsync(
            TestContext.Current.CancellationToken))!);

        await using var createCommand = new NpgsqlCommand(
            "CREATE TABLE reusable_host_marker (id integer PRIMARY KEY)",
            connection);
        await createCommand.ExecuteNonQueryAsync(TestContext.Current.CancellationToken);
    }
}

public sealed class PerTestHostLifecycleTests(PostgresTestFixture fixture)
    : IntegrationTestBase(fixture)
{
    private static WebApplicationFactory<Program>? previousFactory;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
    }

    [Fact]
    public void CustomServices_FirstMethod_ShouldUseAnIsolatedHost()
        => VerifyFactoryIsIsolated();

    [Fact]
    public void CustomServices_SecondMethod_ShouldUseAnIsolatedHost()
        => VerifyFactoryIsIsolated();

    private void VerifyFactoryIsIsolated()
    {
        var previous = Interlocked.Exchange(ref previousFactory, Factory);
        if (previous is not null)
        {
            Assert.NotSame(previous, Factory);
        }
    }
}
