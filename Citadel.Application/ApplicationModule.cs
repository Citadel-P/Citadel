using System.Reflection;
using Hosting.Common.Pipelines;
using Application.Services;
using FluentValidation;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Infrastructure.Services.Abstractions;

namespace Application;

/// <summary>
/// The Application module.
/// </summary>
public static class ApplicationModule
{
    public static IServiceCollection RegisterApplicationModule(this IServiceCollection services)
    {
        services
            .AddMemoryCache()
            .AddSingleton<IJwtService, JwtService>()
            .AddValidatorsFromAssembly(Assembly.GetExecutingAssembly(), includeInternalTypes: true, lifetime: ServiceLifetime.Singleton)
            .AddMediator(options =>
            {
                options.ServiceLifetime = ServiceLifetime.Scoped;
            })
            .AddSingleton(typeof(IPipelineBehavior<,>), typeof(ValidatorBehaviour<,>));

        services
            .AddScoped<IPlatformService, PlatformService>()
            .AddScoped<IContainerService, ContainerService>();

        return services;
    }
}