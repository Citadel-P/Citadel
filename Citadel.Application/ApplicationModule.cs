using System.Reflection;
using Application.Permissions;
using Application.Permissions.Requirements;
using Application.Services;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Pipelines;
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
            .AddMemoryCache()
            .AddSingleton<IJwtService, JwtService>()
            .AddValidatorsFromAssembly(Assembly.GetExecutingAssembly(), includeInternalTypes: true, lifetime: ServiceLifetime.Singleton)
            .AddMediator(options =>
            {
                options.ServiceLifetime = ServiceLifetime.Scoped;
            })
            .AddPermissions()
            .AddSingleton<IPermissionMetadataProvider, GeneratedPermissionMetadataProvider>()
            .AddSingleton(typeof(IPipelineBehavior<,>), typeof(PermissionBehavior<,>))
            .AddSingleton(typeof(IPipelineBehavior<,>), typeof(ValidatorBehavior<,>));

        EnsureDefaultImagesDefinitionsExists();
        return services;
    }

    private static void EnsureDefaultImagesDefinitionsExists()
    {
        if (!File.Exists(Constants.DefaultImagesDefinitionsPath))
        {
            string sourcePath = Path.Combine(AppContext.BaseDirectory, "./Features.Images/default.docker.images.json");
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
}