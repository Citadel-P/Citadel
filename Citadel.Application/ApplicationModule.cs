using Application.Permissions;
using Application.Permissions.Requirements;
using Application.Services;
using Citadel.SourceGen;
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
            .AddMemoryCache()
            .AddSingleton<IJwtService, JwtService>()
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