using Dapper;
using Domain;
using Domain.Configs;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Hosting.Common;
using Hosting.Common.Security;
using Hosting.DockerClient;
using Infrastructure.Connectors;
using Infrastructure.Connectors.AgentConnectors;
using Infrastructure.Connectors.EdgeAgentConnectors;
using Infrastructure.Connectors.LocalConnectors;
using Infrastructure.DockerHub;
using Infrastructure.EdgeAgents;
using Infrastructure.GithubCr;
using Infrastructure.HttpClients.Serializer;
using Infrastructure.Persistence;
using Infrastructure.Repositories;
using Infrastructure.Repositories.DbQueue;
using Infrastructure.Repositories.Security.Grpc;
using Infrastructure.TypeHandlers;
using Infrastructure.Vault;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Npgsql;
using Refit;
using System.Data.Common;

namespace Infrastructure;

public static class InfrastructureModule
{
    private static readonly RefitSettings refitSettings = new() { ContentSerializer = new STJSourceGeneratorSerializer() };

    public static IServiceCollection RegisterInfrastructureModule(this IServiceCollection services, IConfiguration config)
    {
        if (!Helpers.IsDesignTime())
        {
            // Todo: Move this to a hosted service, Init container or similar to avoid delaying startup process
            DbUpgrader.Upgrade(config).GetAwaiter().GetResult();
        }

        return services
            .AddDb(config)
            .AddServices()
            .AddGrpcClients()
            .AddHttpClients()
            .RegisterDockerClient();
    }

    private static IServiceCollection AddGrpcClients(this IServiceCollection services)
    {
        Helpers.GetOrCreatePublicKey();
        services
            .AddSingleton<HubSigningInterceptor>()
            .AddSingleton<CertificateTrust>(sp => CertificateTrust.Load(
                sp.GetRequiredService<IOptions<AgentTransportOptions>>()
                    .Value.CaCertificatePath))
            .AddSingleton<GrpcClientFactory>(sp =>
            {
                var interceptor = sp.GetRequiredService<HubSigningInterceptor>();
                return new GrpcClientFactory(
                    sp.GetRequiredService<IOptions<AgentTransportOptions>>(),
                    sp.GetRequiredService<CertificateTrust>(),
                    interceptor);
            })
            .AddSingleton<IGrpcClientFactory>(sp => sp.GetRequiredService<GrpcClientFactory>())
            .AddSingleton<IPlatformConnectionCache>(sp => sp.GetRequiredService<GrpcClientFactory>())
            .AddGrpc(options =>
            {
                options.MaxReceiveMessageSize = EdgeAgentDefaults.MaxEnvelopePayloadBytes;
                options.MaxSendMessageSize = EdgeAgentDefaults.MaxEnvelopePayloadBytes;
            });

        return services;
    }

    private static IServiceCollection AddDb(this IServiceCollection services, IConfiguration config)
    {
        var connectionString = config.GetConnectionString("Postgres")
            ?? throw new InvalidOperationException("Missing Postgres connection string");

        var builder = new NpgsqlDataSourceBuilder(connectionString);
        if (Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT")?.Equals("Development") == true)
        {
            builder.EnableParameterLogging(true);
        }
        services.AddSingleton(builder.Build());

        services
            .AddScoped<IUnitOfWork, UnitOfWork>()
            .AddScoped<DbConnection>(sp => sp.GetRequiredService<IDbConnectionFactory>().Create())
            .AddSingleton<IDbConnectionFactory, NpgsqlConnectionFactory>();

        return services;
    }

    private static IServiceCollection AddServices(this IServiceCollection services)
        => services
            .AddSingleton<IGitCliRepository, GitCliRepository>()
            .AddSingleton<IAutomationProcessRunner, AutomationProcessRunner>()
            .AddSingleton<IBuildProcessRunner, BuildProcessRunner>()
            .AddSingleton<IResticProcessRunner, ResticProcessRunner>()
            .AddSingleton<IPostgresDumpRunner, PostgresDumpRunner>()
            .AddSingleton<IGitHubCrRepository, GitHubCrRepository>()
            .AddSingleton<IExternalSecretProviderClient, ExternalSecretProviderClient>()
            .AddSingleton<IShoutrrrCliRepository, ShoutrrrCliRepository>()
            .AddSingleton<EdgeAgentSessionRegistry>()
            .AddSingleton<IEdgeAgentSessionTerminator>(provider => provider.GetRequiredService<EdgeAgentSessionRegistry>())
            .AddSingleton<IEdgeAgentCommandRouter, EdgeAgentCommandRouter>()
            .AddSingleton<IDockerHubRegistryRepository, DockerHubRegistryRepository>()
            .AddSingleton<IDbWorkQueue, DbWorkQueue>()
            .AddSingleton<INotificationQueue, NotificationQueue>()
            .AddHostedService<DbWriteWorker>()
            .AddHostedService<NotificationWorker>()
            .AddSingleton<AgentImageConnector>()
            .AddSingleton<LocalImageConnector>()
            .AddSingleton<EdgeImageConnector>()
            .AddSingleton<AgentVolumeConnector>()
            .AddSingleton<LocalVolumeConnector>()
            .AddSingleton<EdgeVolumeConnector>()
            .AddSingleton<AgentNetworkConnector>()
            .AddSingleton<LocalNetworkConnector>()
            .AddSingleton<EdgeNetworkConnector>()
            .AddSingleton<AgentPlatformConnector>()
            .AddSingleton<LocalPlatformConnector>()
            .AddSingleton<AgentContainerConnector>()
            .AddSingleton<EdgePlatformConnector>()
            .AddSingleton<EdgeContainerConnector>()
            .AddSingleton<LocalContainerConnector>()
            .AddSingleton<LocalStackConnector>()
            .AddSingleton<AgentStackConnector>()
            .AddSingleton<EdgeStackConnector>()
            .AddSingleton<AgentDeploymentConnector>()
            .AddSingleton<LocalDeploymentConnector>()
            .AddSingleton<EdgeDeploymentConnector>()
            .AddSingleton(typeof(IConnectorFactory<>), typeof(ConnectorFactory<>))
            .AddConnectorFactory<IImageConnector, AgentImageConnector, LocalImageConnector>(sp => sp.GetRequiredService<EdgeImageConnector>())
            .AddConnectorFactory<IVolumeConnector, AgentVolumeConnector, LocalVolumeConnector>(sp => sp.GetRequiredService<EdgeVolumeConnector>())
            .AddConnectorFactory<INetworkConnector, AgentNetworkConnector, LocalNetworkConnector>(sp => sp.GetRequiredService<EdgeNetworkConnector>())
            .AddConnectorFactory<IStackConnector, AgentStackConnector, LocalStackConnector>(sp => sp.GetRequiredService<EdgeStackConnector>())
            .AddConnectorFactory<IPlatformConnector, AgentPlatformConnector, LocalPlatformConnector>(sp => sp.GetRequiredService<EdgePlatformConnector>())
            .AddConnectorFactory<IContainerConnector, AgentContainerConnector, LocalContainerConnector>(sp => sp.GetRequiredService<EdgeContainerConnector>())
            .AddConnectorFactory<IDeploymentConnector, AgentDeploymentConnector, LocalDeploymentConnector>(sp => sp.GetRequiredService<EdgeDeploymentConnector>());

    /// <summary>
    /// Adds HTTP clients to the service collection.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <returns>The updated service collection.</returns>
    private static IServiceCollection AddHttpClients(this IServiceCollection services)
        => services
            .AddRefitClient<IDockerHubApi>(refitSettings)
                .ConfigureHttpClient(c => c.BaseAddress = new Uri("https://hub.docker.com"))
                .AddPolicyHandler(Configuration.GetRetryPolicy())
            .Services
            .AddRefitClient<IGithubCrApi>(refitSettings)
                .ConfigureHttpClient(c => c.BaseAddress = new Uri("https://api.github.com"))
                .AddPolicyHandler(Configuration.GetRetryPolicy())
            .Services
            .AddSingleton<IVaultKvV2ApiFactory, VaultKvV2ApiFactory>();

    private static void RegisterTypeHandlers()
    {
        // Register custom type handlers to map Guid values to TEXT columns in SQLite, ensuring compatibility between
        // .NET Guid types and SQLite string storage.
        SqlMapper.RemoveTypeMap(typeof(Guid));
        SqlMapper.RemoveTypeMap(typeof(Guid?));
        SqlMapper.AddTypeHandler(new GuidStringHandler());
        SqlMapper.AddTypeHandler(new NullableGuidStringHandler());

        // Json Converters
        SqlMapper.AddTypeHandler(new JsonTypeHandler<IDictionary<string, IReadOnlyList<HostPortBinding>>>(
            ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding));
        SqlMapper.AddTypeHandler(new JsonTypeHandler<PlatformDescriptor>(
           PlatformJsonContext.Default.PlatformDescriptor));
        SqlMapper.AddTypeHandler(new JsonTypeHandler<RegistryConfiguration>(
           RegistryJsonContext.Default.RegistryConfiguration));

        // Enum Converters
        SqlMapper.AddTypeHandler(new StringEnumHandler<Domain.ContainerStateStatus>());
        SqlMapper.AddTypeHandler(new StringEnumHandler<PlatformStatus>()); 
        SqlMapper.AddTypeHandler(new StringEnumHandler<PlatformConnectorType>());
        SqlMapper.AddTypeHandler(new StringEnumHandler<RegistryType>());
    }
}
