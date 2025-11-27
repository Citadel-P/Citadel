using System.Threading.Channels;
using Application.Permissions;
using Application.Permissions.Requirements;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Hosting.Common.Pipelines;
using Hosting.Common.Pipelines.Interfaces;
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

        return services;
    }

    private static IServiceCollection AddServices(this IServiceCollection services)
        => services
            .AddSingleton<IJwtService, JwtService>()
            .AddSingleton<ISyncBarrier, SyncBarrier>()
            .AddSingleton<IPlatformContainerCache, PlatformContainerCache>()
            .AddSingleton<IContainerEventBroadcaster, ContainerEventBroadcaster>()
            .AddSingleton<IPlatformHealthBroadCaster, PlatformHealthBroadCaster>()
            .AddScoped<GitHubConnectorStrategy>()
            .AddScoped<DockerHubConnectorStrategy>()
            .AddScoped<CustomRegistryConnectorStrategy>()
            .AddScoped<IRegistryConnectorResolver, RegistryConnectorResolver>();

    private static IServiceCollection AddSignalRServices(this IServiceCollection services) =>
        services
            .AddSingleton<IStreamSubscriptionResolver, StreamSubscriptionResolver>()
            .AddSingleton<ContainerInfoStreamManager>()
            .AddSingleton<ContainerLogStreamManager>()
            .AddSingleton<ImageStreamManager>()
            .AddSingleton<PlatformStreamManager>()
            .AddSingleton<ContainerStreamManager>()
            .AddSingleton<DockerDaemonStreamManager>()
            .AddSingleton<IImageStreamManager>(s => s.GetRequiredService<ImageStreamManager>())
            .AddSingleton<IPlatformStreamManager>(s => s.GetRequiredService<PlatformStreamManager>())
            .AddSingleton<IContainerStreamManager>(s => s.GetRequiredService<ContainerStreamManager>())
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
            .AddHostedService<ImageSyncJob>()
            .AddHostedService(s => s.GetRequiredService<IPlatformHealthMonitorJob>());
        services
            .AddSingleton<IPlatformHealthMonitorJob, PlatformHealthMonitorJob>()
            .AddSingleton(Channel.CreateBounded<ContainersStatBatch>(Helpers.ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Reader)
            .AddSingleton(Channel.CreateBounded<(Guid Id, PlatformStatsResult Stats)>(Helpers.ChannelDefaultOptions()))
            .AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Writer)
            .AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Reader);

        return services;
    }

    private static IServiceCollection AddPermissions(this IServiceCollection services)
    {
        return
            services
            .AddScoped<IContainerPermissionService, ContainerPermissionService>()
            .AddScoped<IAuthorizationHandler, EditContainerHandler>()
            .AddScoped<IAuthorizationHandler, DeleteContainerHandler>();
    }

}