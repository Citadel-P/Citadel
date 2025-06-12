using System.Diagnostics.CodeAnalysis;
using System.Reflection;
using System.Threading.Channels;
using DbUp;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Infrastructure.Connectors;
using Infrastructure.DockerHub;
using Infrastructure.EntityFramework;
using Infrastructure.GithubCr;
using Infrastructure.HttpClients.Serializer;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.ChangeTracking;
using Microsoft.Extensions.DependencyInjection;
using Refit;
using SQLitePCL;

namespace Infrastructure;

/// <summary>
/// Provides methods to register infrastructure services and configurations.
/// </summary>
public static class InfrastructureModule
{
    private static readonly RefitSettings refitSettings = new() { ContentSerializer = new STJSourceGeneratorSerializer() };

    /// <summary>
    /// Registers the infrastructure module services and configurations.
    /// </summary>
    public static IServiceCollection RegisterInfrastructureModule(this IServiceCollection services)
        => services
            .AddServices()
            .InitializeDb()
            .AddGrpcClients()
            .AddHttpClients()
            .AddBackgroundTasks();

    private static IServiceCollection AddGrpcClients(this IServiceCollection services)
        => services
            .AddSingleton<IGrpcClientFactory, GrpcClientFactory>()
            .AddGrpc().Services;

    private static IServiceCollection AddServices(this IServiceCollection services)
        => services
            .AddSingleton<IGitHubCrService, GitHubCrService>()
            .AddSingleton<IDockerHubService, DockerHubService>()
            .AddSingleton<IContainerConnector, ContainerGrpcConnector>()
            .AddSingleton<IPlatformContainerCache, PlatformContainerCache>();

    private static IServiceCollection AddBackgroundTasks(this IServiceCollection services)
    {

        services
            .AddHostedService<LogCleanupJob>()
            .AddHostedService<DockerDaemonEventJob>()
            .AddHostedService<CleanupStatsJob>()
            .AddHostedService<PlatformSyncJob>()
            .AddHostedService<PlatformsStatsCollectorJob>()
            .AddHostedService<ContainersStatsCollectorJob>()
            .AddHostedService<PlatformsStatsPersistenceJob>()
            .AddHostedService<ContainersStatsPersistenceJob>()
            .AddHostedService<ContainerSyncJob>()
            .AddHostedService(s => s.GetRequiredService<IPlatformHealthMonitorJob>());
        services
            .AddSingleton<IPlatformHealthMonitorJob, PlatformHealthMonitorJob>()
            .AddSingleton(Channel.CreateBounded<ContainersStatBatch>(ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Reader)
            .AddSingleton(Channel.CreateBounded<PlatformStatsBatch>(ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<PlatformStatsBatch>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<PlatformStatsBatch>>().Reader)
            .AddSingleton<IPlatformHealthBroadCaster, PlatformHealthBroadCaster>();
        return services;
    }

    /// <summary>
    /// Adds HTTP clients to the service collection.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The updated service collection.</returns>
    private static IServiceCollection AddHttpClients(this IServiceCollection services)
        => services
            .AddRefitClient<IDockerHubApi>(refitSettings)
                .ConfigureHttpClient(c => c.BaseAddress = new Uri("https://hub.docker.com"))
            .Services
            .AddRefitClient<IGithubCrApi>(refitSettings)
                .ConfigureHttpClient(c => c.BaseAddress = new Uri("https://api.github.com"))
            .Services;

    /// <summary>
    /// Initializes the database.
    /// </summary>
    private static IServiceCollection InitializeDb(this IServiceCollection services)
    {
        Batteries_V2.Init();
        EnsureDatabaseFileExists();
        PerformDatabaseUpgrade();
        EFTrimmingPreserver.PreserveEFCoreTypes();

        return services.AddDbContextPool<ApplicationDbContext>(options =>
        {
            options.UseSqlite(ApplicationContextFactory.ConnectionString, config =>
            {
                config.CommandTimeout(60);
                config.UseQuerySplittingBehavior(QuerySplittingBehavior.SplitQuery);
            });
            options.AddInterceptors(new SqlitePragmaInterceptor());
        });
    }

    private static void EnsureDatabaseFileExists()
    {
        if (!File.Exists(Constants.DbFilePath))
        {
            File.Create(Constants.DbFilePath).Close();
        }
    }

    private static void PerformDatabaseUpgrade()
    {
        var upgrader = DeployChanges.To
            .SqliteDatabase(ApplicationContextFactory.ConnectionString)
            .WithScriptsAndCodeEmbeddedInAssembly(Assembly.GetExecutingAssembly())
            .LogToConsole()
            .Build();

        var result = upgrader.PerformUpgrade();
        if (!result.Successful)
        {
            throw new Exception(result.Error.Message, result.Error);
        }
    }

    internal static BoundedChannelOptions ChannelDefaultOptions() => new (1_000)
    {
        SingleWriter = true,
        SingleReader = true,
        AllowSynchronousContinuations = false,
        FullMode = BoundedChannelFullMode.DropOldest
    };
}

public static class EFTrimmingPreserver
{
    [DynamicDependency(DynamicallyAccessedMemberTypes.PublicConstructors, typeof(EntryCurrentValueComparer<Guid>))]
    public static void PreserveEFCoreTypes() { }
}