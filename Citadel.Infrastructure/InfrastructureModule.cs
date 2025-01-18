using Infrastructure.EntityFramework;
using Microsoft.Extensions.DependencyInjection;
using Refit;
using Polly;
using Polly.Extensions.Http;
using Infrastructure.Services;
using DbUp;
using Microsoft.EntityFrameworkCore;
using System.Reflection;
using Infrastructure.Connected_Services.Serializer;
using Microsoft.AspNetCore.Builder;
using Infrastructure.Services.Grpc;

namespace Infrastructure;

public static class InfrastructureModule
{
    public static IServiceCollection RegisterInfrastructureModule(this IServiceCollection services)
        => services
                .RegisterServices()
                .RegisterHttpClients()
                .InitializeDb()
                .AddGrpc().Services;

    private static IServiceCollection RegisterServices(this IServiceCollection services)
        => services
            .AddScoped<ICacheService, CacheService>()
            .AddScoped<IAgentService, AgentService>();

    private static IServiceCollection RegisterHttpClients(this IServiceCollection services)
    {
        services
            .AddRefitClient<IAgentProxy>(new RefitSettings()
            {
                ContentSerializer = new STJSourceGeneratorSerializer()
            })
            .SetHandlerLifetime(TimeSpan.FromMinutes(10))
            .AddPolicyHandler(option =>
            {
                return HttpPolicyExtensions
                        .HandleTransientHttpError()
                        .WaitAndRetryAsync(
                        [
                            TimeSpan.FromSeconds(1),
                            TimeSpan.FromSeconds(5)
                        ]);
            });
        return services;
    }

    private static IServiceCollection InitializeDb(this IServiceCollection services)
    {
        if (!File.Exists(ApplicationContextFactory.DbFilePath))
        {
            File.Create(ApplicationContextFactory.DbFilePath).Close();
        }

        var upgrader =
                DeployChanges.To
                    .SqliteDatabase(ApplicationContextFactory.ConnectionString)
                    .WithScriptsAndCodeEmbeddedInAssembly(Assembly.GetExecutingAssembly())
                    .LogToConsole()
                    .Build();

        var result = upgrader.PerformUpgrade();
        if (!result.Successful) throw new Exception(result.Error.Message, result.Error);

        return services.AddDbContext<ApplicationDbContext>(c =>
        {
            c.UseSqlite(ApplicationContextFactory.ConnectionString);
        });
    }

    public static void UseInfrastructureModule(this WebApplication app)
    {
        app.MapGrpcService<ContainerGrpcService>();
        app.MapGrpcService<PlatformGrpcService>();
    }
}