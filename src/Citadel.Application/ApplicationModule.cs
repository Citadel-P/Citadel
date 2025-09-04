using System.Threading.Channels;
using Application.Permissions;
using Application.Permissions.Requirements;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Citadel.SourceGen;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Hosting.Common;
using Hosting.Common.ObjectPoolManager;
using Hosting.Common.ObjectPoolManager.Policies;
using Hosting.Common.Pipelines;
using Hosting.Common.Pipelines.Interfaces;
using Microsoft.AspNetCore.Authorization;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.ObjectPool;

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
            .AddPooledObjects()
            .AddBackgroundTasks()
            .AddSignalRServices()
            .AddMediator(options =>
            {
                options.ServiceLifetime = ServiceLifetime.Scoped;
                options.PipelineBehaviors = 
                [
                    typeof(PermissionBehavior<,>),
                    typeof(ValidatorBehavior<,>)
                ];
            })
            .AddPermissions()
            .AddSingleton<IErrorFactoryProvider, ErrorFactoryProvider>()
            .AddSingleton<IValidatorMetadataProvider, ValidatorMetadataProvider>()
            .AddSingleton<IPermissionMetadataProvider, PermissionMetadataProvider>();

        EnsureDefaultImagesDefinitionsExists();
        return services;
    }

    private static IServiceCollection AddServices(this IServiceCollection services)
        => services
            .AddSingleton<IJwtService, JwtService>()
            .AddSingleton<IPlatformContainerCache, PlatformContainerCache>()
            .AddSingleton<IContainerEventBroadcaster, ContainerEventBroadcaster>()
            .AddSingleton<IPlatformHealthBroadCaster, PlatformHealthBroadCaster>()
            
            .AddScoped<GitHubConnectorStrategy>()
            .AddScoped<DockerHubConnectorStrategy>()
            .AddScoped<IRegistryConnectorResolver, RegistryConnectorResolver>();

    private static IServiceCollection AddSignalRServices(this IServiceCollection services) =>
        services
            .AddSingleton<IStreamSubscriptionResolver, StreamSubscriptionResolver>()
            .AddSingleton<ContainerInfoStreamManager>()
            .AddSingleton<ContainerLogStreamManager>()
            .AddSingleton<PlatformsStreamManager>()
            .AddSingleton<ContainersStreamManager>()
            .AddSingleton<DockerDaemonStreamManager>()
            .AddSingleton<IPlatformsStreamManager>(s => s.GetRequiredService<PlatformsStreamManager>())
            .AddSingleton<IContainersStreamManager>(s => s.GetRequiredService<ContainersStreamManager>())
            .AddSingleton<IDockerDaemonStreamManager>(s => s.GetRequiredService<DockerDaemonStreamManager>());

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
            .AddSingleton(Channel.CreateBounded<(Guid Id, PooledHandle<PlatformStatsResult> Stats)>(ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PooledHandle<PlatformStatsResult> Stats)>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PooledHandle<PlatformStatsResult> Stats)>>().Reader);

        return services;
    }

    private static IServiceCollection AddPooledObjects(this IServiceCollection services)
    {
        services
            .AddDomainPooledObjects()
            .AddSingleton<IPooledObjectPolicy<List<PlatformStat>>, ListPoolPolicy<PlatformStat>>()
            .AddSingleton<IPooledObjectPolicy<List<ContainerStat>>, ListPoolPolicy<ContainerStat>>()
            .AddSingleton<IPooledObjectPolicy<Dictionary<string, DockerContainerStat>>, DictionaryPooledPolicy<string, DockerContainerStat>>()
            .AddSingleton<IPooledObjectPolicy<List<PooledHandle<PlatformStatsResult>>>, ObjectPoolPolicy<List<PooledHandle<PlatformStatsResult>>>>();

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