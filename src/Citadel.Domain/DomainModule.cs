using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Hosting.Common.ObjectPoolManager;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.ObjectPool;

namespace Domain;

public static class DomainModule
{
    public static IServiceCollection AddDomainPooledObjects(this IServiceCollection services)
    {
        services
            .AddSingleton<IPooledObjectPolicy<PlatformStatsResult>>(_ => new FuncObjectPolicy<PlatformStatsResult>(() => new PlatformStatsResult()))
            .AddSingleton<IPooledObjectPolicy<DockerContainerStat>>(_ => new FuncObjectPolicy<DockerContainerStat>(() => new DockerContainerStat()))
            .AddSingleton<IPooledObjectPolicy<DockerContainer>>(_ => new FuncObjectPolicy<DockerContainer>(() => new DockerContainer()))
            .AddSingleton<IPooledObjectPolicy<ContainerStat>>(_ => new FuncObjectPolicy<ContainerStat>(() => new ContainerStat()))
            .AddSingleton<IPooledObjectPolicy<PlatformStat>>(_ => new FuncObjectPolicy<PlatformStat>(() => new PlatformStat()));

        return services;
    }
}
