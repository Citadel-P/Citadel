using Domain;
using Microsoft.Extensions.DependencyInjection;

namespace Infrastructure;

public static partial class Extensions
{
    public static IServiceCollection AddConnectorFactory<TService, TAgent, TLocal>(this IServiceCollection services)
        where TService : class
        where TAgent : class, TService
        where TLocal : class, TService
    {
        services.AddSingleton<Func<PlatformConnectorType, TService>>(provider => key =>
        {
            return key switch
            {
                PlatformConnectorType.Agent => provider.GetRequiredService<TAgent>(),
                PlatformConnectorType.Local => provider.GetRequiredService<TLocal>(),
                _ => throw new ArgumentOutOfRangeException(nameof(key), key, null)
            };
        });

        return services;
    }
}
