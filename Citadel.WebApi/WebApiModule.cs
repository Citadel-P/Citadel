using System.Text;
using System.Text.Json.Serialization;
using Hosting.Converters;
using Hosting.OpenApi;
using Infrastructure.Services.Abstractions;
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
            .AddOpenApi(cfg =>
            {
                cfg.AddSchemaTransformer<EnumSchemaFilter>();
                cfg.AddDocumentTransformer<ServerTransformer>();
                cfg.AddDocumentTransformer<BearerSecuritySchemeTransformer>();
                cfg.AddOperationTransformer<AddCookieOperationTransformer>();
                cfg.AddOperationTransformer<ProduceCookieOperationTransformer>();
            })
            .AddCors();

        services.AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
            .AddJwtBearer(options =>
            {
                string key = string.IsNullOrEmpty(configuration["Jwt:Key"])
                                        ? Application.Utils.Helpers.GetJwtSecretFromFile()
                                        : configuration["Jwt:Key"];

                options.TokenValidationParameters = new TokenValidationParameters()
                {
                    IssuerSigningKey = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(key)),
                    ValidIssuer = configuration["Jwt:Issuer"],
                    ValidAudience = configuration["Jwt:Audience"],
                    ValidateIssuerSigningKey = true,
                    ValidateIssuer = true,
                    ValidateAudience = true,
                    ValidateLifetime = true,
                    ClockSkew = TimeSpan.Zero
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

        services.AddAuthorization();
        services.AddSingleton<IAuthorizationMiddlewareResultHandler, AuthorizationResultHandler>();
        services.AddSignalRDependencies();
        return services;
    }

    public static WebApplication UseWebApiModule(this WebApplication app)
    {
        var cors = app.Configuration.GetSection("Cors").Get<string[]>();
        if (cors != null && cors.Length != 0)
        {
            app.UseCors(policy =>
            {
                policy
                    .AllowCredentials()
                    .WithOrigins(cors)
                    .AllowAnyMethod()
                    .AllowAnyHeader();
            });
        }

        app.UseAuthentication();
        app.UseAuthorization();

        app.MapPublicEndpoints();

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

        services.AddSignalR().AddJsonProtocol(c => 
        {
            c.PayloadSerializerOptions.TypeInfoResolverChain.Add(SignalRSerializeContext.Default);
            c.PayloadSerializerOptions.DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull;
            c.PayloadSerializerOptions.Converters.Add(new JsonStringEnumConverter());
            c.PayloadSerializerOptions.Converters.Add(new DatetimeOffsetConverter());
            c.PayloadSerializerOptions.Converters.Add(new DatetimeConverter());
        });
    }
}