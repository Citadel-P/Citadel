using Contracts.Broker.EventMessaging.Options;
using Contracts.Broker.EventMessaging.Subscriber;
using Contracts.Broker.Models;
using EasyNetQ.AutoSubscribe;
using Infrastructure.EntityFramework;
using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Refit;
using Polly;
using Polly.Extensions.Http;
using Infrastructure.Services.AutoSubscribers;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using DbUp;
using Microsoft.EntityFrameworkCore;
using System.Reflection;

namespace Infrastructure;

public static class InfrastructureModule
{
    public static IServiceCollection RegisterInfrastructureModule(this IServiceCollection services, IConfiguration config)
        => services
                .RegisterServices()
                .RegisterHttpClients()
                .RegisterAutoSubscribers(config)
                .InitializeDb();

    public static WebApplication UseInfrastructureModule(this WebApplication builder)
        => builder
             .UseAutoSubscriber();

    private static IServiceCollection RegisterServices(this IServiceCollection services)
        => services
            .AddScoped<ICacheService, CacheService>()
            .AddScoped<IAgentService, AgentService>();

    private static IServiceCollection RegisterHttpClients(this IServiceCollection services)
    {
        services
            .AddRefitClient<IAgentProxy>()
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

    private static IServiceCollection RegisterAutoSubscribers(this IServiceCollection services, IConfiguration config)
    {
        var busConfig = config.GetSection("BusConfiguration").Get<BusConfigurationOptions>() ??
                                throw new ArgumentNullException("Broker configuration is missing.");
        return
            services
                .RegisterAutoSubscriber(busConfig)
                .AddScoped<IConsumeAsync<SystemInfoMessage>, PlatformAutoSubscriber>()
                .AddScoped<IConsumeAsync<ContainerListMessage>, ContainerAutoSubscriber>()
                .AddScoped<IConsumeAsync<ContainerEventMessage>, ContainerAutoSubscriber>()
                .AddScoped<IConsumeAsync<ContainerLogMessage>, ContainerAutoSubscriber>();
    }

    private static IServiceCollection InitializeDb(this IServiceCollection services)
    {
        if (!File.Exists(ApplicationContextFactory.DbFilePath))
        {
            File.Create(ApplicationContextFactory.DbFilePath).Close();
        }

        var upgrader =
                DeployChanges.To
                    .SQLiteDatabase(ApplicationContextFactory.ConnectionString)
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
}