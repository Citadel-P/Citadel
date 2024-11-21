using System.Text;
using Application.Services.Abstractions;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using Microsoft.IdentityModel.Tokens;
using WebApi.Hubs;
using WebApi.Middlewares;

namespace WebApi;

internal static class WebApiModule
{
    public static IServiceCollection RegisterWebApiModule(this IServiceCollection services, IConfiguration configuration)
    {
        services.AddCors();

        services
            .AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
            .AddJwtBearer(options =>
            {
                string key = string.IsNullOrEmpty(configuration["Jwt:Key"])
                                        ? Application.Helpers.GetJwtSecretFromFile()
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
                    "https://localhost:8000",
                    "http://localhost:8000",
                    "https://localhost:5173", // Client dev proxies,
                    "http://localhost:5173")  // it's better to remove them in prod
                .SetIsOriginAllowedToAllowWildcardSubdomains()
                .AllowAnyMethod()
                .AllowAnyHeader();
        });

        app.UseAuthentication();
        app.UseAuthorization();

        app.MapControllers();

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
        services.AddSignalR().AddMessagePackProtocol();
    }
}