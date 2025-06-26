using System.Diagnostics.CodeAnalysis;
using System.Reflection;
using DbUp;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.DockerClient;
using Infrastructure.Connectors;
using Infrastructure.Connectors.AgentConnectors;
using Infrastructure.Connectors.LocalConnectors;
using Infrastructure.DockerHub;
using Infrastructure.EntityFramework;
using Infrastructure.GithubCr;
using Infrastructure.HttpClients.Serializer;
using Infrastructure.Repositories;
using Infrastructure.Services;
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
            .RegisterDockerClient();

    private static IServiceCollection AddGrpcClients(this IServiceCollection services)
        => services
            .AddSingleton<IGrpcClientFactory, GrpcClientFactory>()
            .AddGrpc().Services;

    private static IServiceCollection AddServices(this IServiceCollection services)
        => services
            .AddSingleton<IGitHubCrRepository, GitHubCrRepository>()
            .AddSingleton<IDockerHubRegistryRepository, DockerHubRegistryRepository>()
            .AddSingleton<AgentImageConnector>()
            .AddSingleton<LocalImageConnector>()
            .AddSingleton<AgentVolumeConnector>()
            .AddSingleton<LocalVolumeConnector>()
            .AddSingleton<AgentNetworkConnector>()
            .AddSingleton<LocalNetworkConnector>()
            .AddSingleton<AgentPlatformConnector>()
            .AddSingleton<LocalPlatformConnector>()
            .AddSingleton<AgentContainerConnector>()
            .AddSingleton<LocalContainerConnector>()
            .AddSingleton(typeof(IConnectorFactory<>), typeof(ConnectorFactory<>))
            .AddConnectorFactory<IImageConnector, AgentImageConnector, LocalImageConnector>()
            .AddConnectorFactory<IVolumeConnector, AgentVolumeConnector, LocalVolumeConnector>()
            .AddConnectorFactory<INetworkConnector, AgentNetworkConnector, LocalNetworkConnector>()
            .AddConnectorFactory<IPlatformConnector, AgentPlatformConnector, LocalPlatformConnector>()
            .AddConnectorFactory<IContainerConnector, AgentContainerConnector, LocalContainerConnector>();

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
        services.AddScoped<IUnitOfWork, UnitOfWork>();
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
        if (!System.IO.File.Exists(Constants.DbFilePath))
        {
            System.IO.File.Create(Constants.DbFilePath).Close();
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