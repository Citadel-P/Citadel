using Infrastructure.EntityFramework;
using Microsoft.Extensions.DependencyInjection;
using Infrastructure.Services;
using DbUp;
using Microsoft.EntityFrameworkCore;
using System.Reflection;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using Microsoft.Extensions.Configuration;

namespace Infrastructure;

public static class InfrastructureModule
{
    public static IServiceCollection RegisterInfrastructureModule(this IServiceCollection services, IConfiguration configuration)
        => services
                .RegisterServices()
                .InitializeDb()
                .AddGrpcClients()
                .AddTaskJobs(configuration);

    private static IServiceCollection RegisterServices(this IServiceCollection services)
        => services
            .AddScoped<ICacheService, CacheService>();

    private static IServiceCollection AddGrpcClients(this IServiceCollection services)
        => services
        .AddSingleton<IGrpcClientFactory, GrpcClientFactory>()
        .AddGrpc().Services;

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

        return services.AddDbContextPool<ApplicationDbContext>(c =>
        {
            c.UseSqlite(ApplicationContextFactory.ConnectionString);
        });
    }
}