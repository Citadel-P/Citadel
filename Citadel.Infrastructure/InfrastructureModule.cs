using System.Reflection;
using Dapper;
using DbUp;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Hosting.Common;
using Hosting.DockerClient;
using Infrastructure.Connectors;
using Infrastructure.Connectors.AgentConnectors;
using Infrastructure.Connectors.LocalConnectors;
using Infrastructure.DockerHub;
using Infrastructure.GithubCr;
using Infrastructure.HttpClients.Serializer;
using Infrastructure.Persistence;
using Infrastructure.Repositories;
using Infrastructure.Services;
using Infrastructure.TypeHandlers;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Refit;
using SQLitePCL;

namespace Infrastructure;

/// <summary>
/// Provides methods to register infrastructure services and configurations.
/// </summary>
public static class InfrastructureModule
{
    internal const string connectionString = $"Data Source={Constants.DbFilePath};Mode=ReadWriteCreate;Pooling=True;";
    private static readonly RefitSettings refitSettings = new() { ContentSerializer = new STJSourceGeneratorSerializer() };

    /// <summary>
    /// Registers the infrastructure module services and configurations.
    /// </summary>
    public static IServiceCollection RegisterInfrastructureModule(this IServiceCollection services, IWebHostEnvironment environment)
        => services
            .AddServices()
            .InitializeDb(environment)
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
    private static IServiceCollection InitializeDb(this IServiceCollection services, IWebHostEnvironment environment)
    {
        Batteries_V2.Init();
        RegisterTypeHandlers();

        // Run the migration logic directly if not in a test environment
        if (!environment.IsEnvironment("IntegrationTests"))
        {
            EnsureDatabaseFileExists();
            PerformDatabaseUpgrade();
        }
        
        return services
            .AddScoped<IUnitOfWork, UnitOfWork>()
            .AddScoped(sp => sp.GetRequiredService<IDbConnectionFactory>().Create())
            .AddSingleton<IDbConnectionFactory>(new SqliteConnectionFactory(connectionString));
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
            .SqliteDatabase(connectionString)
            .WithScriptsAndCodeEmbeddedInAssembly(Assembly.GetExecutingAssembly())
            .LogToConsole()
            .Build();

        var result = upgrader.PerformUpgrade();
        if (!result.Successful)
        {
            throw new Exception(result.Error.Message, result.Error);
        }
    }

    private static void RegisterTypeHandlers()
    {
        // Register custom type handlers to map Guid values to TEXT columns in SQLite, ensuring compatibility between
        // .NET Guid types and SQLite string storage.
        SqlMapper.RemoveTypeMap(typeof(Guid));
        SqlMapper.RemoveTypeMap(typeof(Guid?));
        SqlMapper.AddTypeHandler(new GuidStringHandler());
        SqlMapper.AddTypeHandler(new NullableGuidStringHandler());

        // Json Converters
        SqlMapper.AddTypeHandler(new JsonTypeHandler<List<ContainerPort>>(
            ContainerPortsContext.Default.ListContainerPort));
        SqlMapper.AddTypeHandler(new JsonTypeHandler<PlatformDescriptor>(
           PlatformJsonContext.Default.PlatformDescriptor));
        SqlMapper.AddTypeHandler(new JsonTypeHandler<RegistryConfigurationBase>(
           RegistryJsonContext.Default.RegistryConfigurationBase));

        // Enum Converters
        SqlMapper.AddTypeHandler(new StringEnumHandler<Domain.ContainerStateStatus>());
        SqlMapper.AddTypeHandler(new StringEnumHandler<PlatformStatus>()); 
        SqlMapper.AddTypeHandler(new StringEnumHandler<PlatformConnectorType>());
        SqlMapper.AddTypeHandler(new StringEnumHandler<RegistryType>());
    }
}
