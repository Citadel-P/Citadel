using System.Diagnostics.CodeAnalysis;
using System.Reflection;
using System.Threading.Channels;
using Agent.Server.GPlatform;
using DbUp;
using Hosting.Common;
using Infrastructure.DockerHub;
using Infrastructure.Entities;
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
    /// <param name="services">The service collection.</param>
    /// <returns>The updated service collection.</returns>
    public static IServiceCollection RegisterInfrastructureModule(this IServiceCollection services)
        => services
            .InitializeDb()
            .AddGrpcClients()
            .AddHttpClients()
            .AddBackgroundTasks();

    /// <summary>
    /// Adds gRPC clients to the service collection.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The updated service collection.</returns>
    private static IServiceCollection AddGrpcClients(this IServiceCollection services)
        => services
            .AddSingleton<IGrpcClientFactory, GrpcClientFactory>()
            .AddGrpc().Services;

    private static IServiceCollection AddBackgroundTasks(this IServiceCollection services)
    {
        
        services
            .AddHostedService<PlatformStateSyncJob>()
            .AddHostedService(s => s.GetRequiredService<IPlatformHealthMonitorJob>())
            .AddHostedService(s => s.GetRequiredService<IContainersStatsReaderJob>())
            .AddHostedService(s => s.GetRequiredService<IPlatformsStatsReaderJob>())
            .AddHostedService(s => s.GetRequiredService<IDaemonEventJob>())
            .AddHostedService<ContainersStatsWriterJob>()
            .AddHostedService<PlatformsStatsWriterJob>()
            .AddHostedService<CleanupStatsJob>()
            .AddHostedService<LogCleanupJob>();
        services
            .AddSingleton<IDaemonEventJob, DaemonEventJob>()
            .AddSingleton<IPlatformsStatsReaderJob, PlatformsStatsReaderJob>()
            .AddSingleton<IContainersStatsReaderJob, ContainersStatsReaderJob>()
            .AddSingleton<IPlatformHealthMonitorJob, PlatformHealthMonitorJob>()
            .AddSingleton(Channel.CreateUnbounded<ContainersStatBatch>(
                new UnboundedChannelOptions
                {
                    SingleWriter = true,
                    AllowSynchronousContinuations = false
                }))
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Reader)
            .AddSingleton(Channel.CreateUnbounded<PlatformStatsBatch>(
                new UnboundedChannelOptions
                {
                    SingleWriter = true,
                    AllowSynchronousContinuations = false
                }))
            .AddSingleton(s => s.GetRequiredService<Channel<PlatformStatsBatch>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<PlatformStatsBatch>>().Reader)
            .AddSingleton(Channel.CreateUnbounded<GrpcServiceHealth>(
                new UnboundedChannelOptions
                {
                    SingleWriter = true,
                    AllowSynchronousContinuations = false
                }))
            .AddSingleton(s => s.GetRequiredService<Channel<GrpcServiceHealth>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<GrpcServiceHealth>>().Reader);
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
    /// <param name="services">The service collection.</param>
    /// <returns>The updated service collection.</returns>
    private static IServiceCollection InitializeDb(this IServiceCollection services)
    {
        EnsureDatabaseFileExists();
        PerformDatabaseUpgrade();
        EFTrimmingPreserver.PreserveEFCoreTypes();

        return services.AddDbContextPool<ApplicationDbContext>(c =>
        {
            c.UseSqlite(ApplicationContextFactory.ConnectionString, config =>
            {
                config.CommandTimeout(60);
                config.UseQuerySplittingBehavior(QuerySplittingBehavior.SplitQuery);
            });
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
}

public static class EFTrimmingPreserver
{
    [DynamicDependency(DynamicallyAccessedMemberTypes.PublicConstructors, typeof(EntryCurrentValueComparer<Guid>))]
    public static void PreserveEFCoreTypes() { }
}