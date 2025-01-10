using System.Text;
using Application.Services.Abstractions;
using Hosting.OpenApi;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using Microsoft.IdentityModel.Tokens;
using WebApi.Hubs;
using WebApi.Middlewares;
using WebApi.Routes;

namespace WebApi;

internal static class WebApiModule
{
    public static IServiceCollection RegisterWebApiModule(this IServiceCollection services, IConfiguration configuration)
    {
        services
            .AddOpenApi(Constants.PublicApiV1, cfg =>
            {
                cfg.AddSchemaTransformer<EnumSchemaFilter>();
                cfg.AddDocumentTransformer<ServerTransformer>();
                cfg.AddOperationTransformer<ProblemDetailDocumentFilter>();
                cfg.AddDocumentTransformer<BearerSecuritySchemeTransformer>();
            })
            .AddOpenApi(Constants.InternalApiV1, cfg =>
            {
                cfg.AddSchemaTransformer<EnumSchemaFilter>();
                cfg.AddDocumentTransformer<ServerTransformer>();
                cfg.AddOperationTransformer<ProblemDetailDocumentFilter>();
            })
            .AddCors();

        services
            .AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
            .AddJwtBearer(options =>
            {
                string key = string.IsNullOrEmpty(configuration["Jwt:Key"])
                                        ? Application.Utils.Helpers.GetJwtSecretFromFile()
                                        : configuration["Jwt:Key"];

                options.TokenValidationParameters = new TokenValidationParameters()
                {
                    ValidIssuer = configuration["Jwt:Issuer"],
                    ValidAudience = configuration["Jwt:Audience"],
                    IssuerSigningKey = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(key)),
                    ValidateIssuer = true,
                    ValidateAudience = true,
                    ValidateLifetime = true,
                    ValidateIssuerSigningKey = true
                };

                options.Events = new JwtBearerEvents()
                {
                    // We have to hook the OnMessageReceived event in order to
                    // allow the JWT authentication handler to read the access
                    // token from the query string when a WebSocket or
                    // Server-Sent Events request comes in.
                    OnMessageReceived = context =>
                    {
                        var accessToken = context.Request.Query["access_token"];

                        var path = context.HttpContext.Request.Path;
                        if (!string.IsNullOrEmpty(accessToken) && path.StartsWithSegments("/hubs"))
                        {
                            context.Token = accessToken;
                        }
                        return Task.CompletedTask;
                    }
                };
            });
        services.AddSingleton<IAuthorizationMiddlewareResultHandler, AuthorizationResultHandler>();
        services.AddSignalRDependencies();
        return services;
    }

    public static WebApplication UseWebApiModule(this WebApplication app)
    {
        app.UseCors(
        policy =>
        {
            policy
                .AllowCredentials()
                .WithOrigins(
                    "http://localhost:8000",
                    "http://localhost:5173")
                .SetIsOriginAllowedToAllowWildcardSubdomains()
                .AllowAnyMethod()
                .AllowAnyHeader();
        });

        app.UseAuthentication();
        app.UseAuthorization();

        app.MapPublicEndpoints();
        app.MapInternalEndpoints();

        app.MapHub<ContainerHub>("/hubs/container");
        app.MapHub<PlatformHub>("/hubs/platform");

        return app;
    }

    private static void AddSignalRDependencies(this IServiceCollection services)
    {
        // We need to register this factories in order to use the view models
        services.AddSingleton<IContainerHubDispatcher>(provider =>
        {
            var context = provider.GetRequiredService<IHubContext<ContainerHub, ITypedContainerHub>>();
            return new ContainerHubDispatcher(context);
        });
        services.AddSingleton<IPlatformHubDispatcher>(provider =>
        {
            var context = provider.GetRequiredService<IHubContext<PlatformHub, ITypedPlatformHub>>();
            return new PlatformHubDispatcher(context);
        });
        services.AddSignalR();
    }
}