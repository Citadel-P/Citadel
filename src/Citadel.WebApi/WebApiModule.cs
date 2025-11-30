using Application.Services.Abstractions;
using Hosting.OpenApi;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http.Connections;
using Microsoft.AspNetCore.SignalR;
using Microsoft.IdentityModel.Tokens;
using Microsoft.OpenApi;
using Nerdbank.MessagePack;
using Nerdbank.MessagePack.SignalR;
using System.Text;
using System.Text.Json.Serialization;
using WebApi.Hubs;
using WebApi.Middlewares;
using WebApi.Routes;
using static Nerdbank.MessagePack.OptionalConverters;

namespace WebApi;

internal static class WebApiModule
{
    public static IServiceCollection RegisterWebApiModule(this IServiceCollection services, IConfiguration configuration)
    {
        services
            .AddOpenApi(options =>
            {
                options.OpenApiVersion = OpenApiSpecVersion.OpenApi3_1; 
                options.AddDocumentTransformer<ServerTransformer>();
                options.AddDocumentTransformer<BearerSecuritySchemeTransformer>();
                options.AddOperationTransformer<AddCookieOperationTransformer>();
                options.AddOperationTransformer<ProduceCookieOperationTransformer>();
                options.AddOperationTransformer<ExampleOperationTransformer>();
            })
            .AddCors();

        services.AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
            .AddJwtBearer(options =>
            {
                var key = (string.IsNullOrEmpty(configuration["Jwt:Key"])
                                        ? Hosting.Common.Helpers.GetJwtSecretFromFile()
                                        : configuration["Jwt:Key"]) 
                                        ?? throw new ArgumentNullException("Jwt:Key is missing from configuration");
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

        app.MapHub<ApplicationHub>("/hubs/global", HttpConnectionDispatcherOptions);
        return app;
    }

    private static void HttpConnectionDispatcherOptions(HttpConnectionDispatcherOptions options)
    {
        options.LongPolling.PollTimeout = TimeSpan.FromSeconds(30);
        options.Transports = HttpTransportType.WebSockets | HttpTransportType.LongPolling;
    }

    private static void AddSignalRDependencies(this IServiceCollection services)
    {
        // We need to register this factories in order to use the view models
        services
            .AddSingleton<IApplicationHubDispatcher>(provider =>
            {
                var context = provider.GetRequiredService<IHubContext<ApplicationHub>>();
                return new ApplicationHubDispatcher(context);
            });

        services.AddSignalR().AddMessagePackProtocol(SignalRMessagePackContext.GeneratedTypeShapeProvider, new MessagePackSerializer
        {
            SerializeEnumValuesByName = true,
            PropertyNamingPolicy = MessagePackNamingPolicy.CamelCase,
            DerivedTypeUnions = [DerivedTypesMapping.PlatformDescriptorMappings],
        }
        .WithGuidConverter(GuidStringFormat.StringD)
        .WithAssumedDateTimeKind(DateTimeKind.Utc));
    }

    internal static void AddGenericEnumConverters(this IList<JsonConverter> converters)
    {
        foreach (var converter in Citadel.SourceGen.GeneratedJsonConverters.All)
        {
            converters.Add(converter);
        }
    }
}

// Required for integration tests to work properly
public partial class Program { }
