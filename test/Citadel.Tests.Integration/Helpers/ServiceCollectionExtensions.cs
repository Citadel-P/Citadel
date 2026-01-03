using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Helpers;

internal static class ServiceCollectionExtensions
{
    public static IServiceCollection ReplaceService<T>(this IServiceCollection services, T instance) where T : class
    {
        RemoveService<T>(services);
        services.AddSingleton(instance);
        return services;
    }

    public static void ReplaceService<T>(this IServiceCollection services, Func<IServiceProvider, T> factory) where T : class
    {
        RemoveService<T>(services);
        services.AddScoped(factory);
    }

    public static void RemoveService<T>(this IServiceCollection services)
    {
        var descriptor = services.FirstOrDefault(d => d.ServiceType == typeof(T));
        if (descriptor != null)
        {
            services.Remove(descriptor);
        }
    }
}
