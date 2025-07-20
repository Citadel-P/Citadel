using System.Threading.Channels;
using Application.Permissions;
using Application.Permissions.Requirements;
using Application.Services;
using Application.TaskJobs;
using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Hosting.Common.Pipelines;
using Hosting.Common.Pipelines.Interfaces;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.Extensions.DependencyInjection;

namespace Application;

/// <summary>
/// The Application module.
/// </summary>
public static class ApplicationModule
{
    public static IServiceCollection RegisterApplicationModule(this IServiceCollection services)
    {
        services
            .AddServices()
            .AddBackgroundTasks()
            .AddMediator(options =>
            {
                options.ServiceLifetime = ServiceLifetime.Scoped;
            })
            .AddPermissions()
            .AddSingleton<IErrorFactoryProvider, ErrorFactoryProvider>()
            .AddSingleton<IValidatorMetadataProvider, ValidatorMetadataProvider>()
            .AddSingleton<IPermissionMetadataProvider, PermissionMetadataProvider>()
            .AddSingleton(typeof(IPipelineBehavior<,>), typeof(PermissionBehavior<,>))
            .AddSingleton(typeof(IPipelineBehavior<,>), typeof(ValidatorBehavior<,>));

        EnsureDefaultImagesDefinitionsExists();
        return services;
    }

    private static IServiceCollection AddServices(this IServiceCollection services)
        => services
            .AddSingleton<IJwtService, JwtService>()
            .AddSingleton<IPlatformContainerCache, PlatformContainerCache>()
            .AddSingleton<IPlatformHealthBroadCaster, PlatformHealthBroadCaster>()
            .AddScoped<GitHubConnectorStrategy>()
            .AddScoped<DockerHubConnectorStrategy>()
            .AddScoped<IRegistryConnectorResolver, RegistryConnectorResolver>();

    private static IServiceCollection AddBackgroundTasks(this IServiceCollection services)
    {
        services
            .AddHostedService<DockerDaemonEventJob>()
            .AddHostedService<CleanupStatsJob>()
            .AddHostedService<PlatformSyncJob>()
            .AddHostedService<PlatformStatsStreamerJob>()
            .AddHostedService<ContainerStatsStreamerJob>()
            .AddHostedService<PlatformStatsWriterJob>()
            .AddHostedService<ContainerStatsWriterJob>()
            .AddHostedService<ContainerSyncJob>()
            .AddHostedService(s => s.GetRequiredService<IPlatformHealthMonitorJob>());
        services
            .AddSingleton<IPlatformHealthMonitorJob, PlatformHealthMonitorJob>()
            .AddSingleton(Channel.CreateBounded<ContainersStatBatch>(ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Reader)
            .AddSingleton(Channel.CreateBounded<(Guid Id, PlatformStatsResult Stats)>(ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Reader);

        return services;
    }

    private static void EnsureDefaultImagesDefinitionsExists()
    {
        if (!File.Exists(Constants.DefaultImagesDefinitionsPath))
        {
            string sourcePath = Path.Combine(AppContext.BaseDirectory, "Features.Images/default.docker.images.json");
            try
            {
                File.Copy(sourcePath, Constants.DefaultImagesDefinitionsPath);
            }
            catch (Exception ex)
            {
                Console.WriteLine(ex);
            }
        }
    }

    private static IServiceCollection AddPermissions(this IServiceCollection services)
    {
        return
            services
            .AddScoped<IContainerPermissionService, ContainerPermissionService>()
            .AddScoped<IAuthorizationHandler, EditContainerHandler>()
            .AddScoped<IAuthorizationHandler, DeleteContainerHandler>();
    }

    internal static BoundedChannelOptions ChannelDefaultOptions() => new(1_000)
    {
        SingleWriter = true,
        SingleReader = true,
        AllowSynchronousContinuations = false,
        FullMode = BoundedChannelFullMode.DropOldest
    };
}