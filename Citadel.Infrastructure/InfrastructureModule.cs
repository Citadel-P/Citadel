using System.Diagnostics.CodeAnalysis;
using System.Reflection;
using DbUp;
using Hosting.Common;
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
            .AddHostedService<ContainersInfoJob>()
            .AddHostedService<PlatformInfoJob>()
            .AddHostedService<DaemonEventJob>()
            .AddHostedService<LogCleanupJob>();
        services.AddSingleton<IDaemonEventJob, DaemonEventJob>();
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