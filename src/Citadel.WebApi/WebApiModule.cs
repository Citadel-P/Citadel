using Application.Configs;
using Application.Services;
using Application.Services.Abstractions;
using Hosting.Common;
using Hosting.Common.Extensions;
using Hosting.OpenApi;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.DataProtection;
using Microsoft.AspNetCore.Http.Connections;
using Microsoft.AspNetCore.SignalR;
using Microsoft.IdentityModel.Tokens;
using Microsoft.OpenApi;
using Nerdbank.MessagePack;
using Nerdbank.MessagePack.SignalR;
using System.Text;
using System.Text.Json.Serialization;
using System.Threading.RateLimiting;
using Infrastructure.EdgeAgents;
using WebApi.OpenApi;
using WebApi.Hubs;
using WebApi.Middlewares;
using WebApi.Routes;
using static Nerdbank.MessagePack.OptionalConverters;

namespace WebApi;

internal static class WebApiModule
{
    public static IServiceCollection RegisterWebApiModule(this IServiceCollection services, IConfiguration configuration)
    {
        Directory.CreateDirectory(Constants.DataProtectionKeysPath);

        services
            .AddDataProtection()
            .PersistKeysToFileSystem(new DirectoryInfo(Constants.DataProtectionKeysPath));

        services
            .AddOpenApi(options =>
            {
                options.OpenApiVersion = OpenApiSpecVersion.OpenApi3_1; 
                options.AddSchemaTransformer<DateTimeOffsetSchemaTransformer>();
                options.AddDocumentTransformer<ServerTransformer>();
                options.AddDocumentTransformer<BearerSecuritySchemeTransformer>();
                options.AddDocumentTransformer<KnownEnumSchemaDocumentTransformer>();
                options.AddOperationTransformer<AddCookieOperationTransformer>();
                options.AddOperationTransformer<ProduceCookieOperationTransformer>();
                options.AddOperationTransformer<ExampleOperationTransformer>();
                options.AddOperationTransformer<RateLimitOperationTransformer>();
            })
            .AddSingleton<IAutomationApiEndpointCatalog, EndpointDataSourceAutomationApiEndpointCatalog>()
            .AddCors();

        services
            .AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
            .AddJwtBearer(options =>
            {
                var key = (string.IsNullOrEmpty(configuration["Jwt:Key"])
                                        ? Helpers.GetJwtSecretFromFile()
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

        app.UseMiddleware<SetupRequiredMiddleware>();
        app.UseAuthentication();
        app.UseMiddleware<AutomationRunTokenSafetyMiddleware>();
        app.UseAuthorization();

        app.MapPublicEndpoints();
        app.MapEdgeAgentGrpcService();

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
            DerivedTypeUnions =
            [
                DerivedTypesMapping.PlatformDescriptorMappings,
                DerivedTypesMapping.BackupRepositorySpecMappings,
                DerivedTypesMapping.BackupSourceSpecMappings,
                DerivedTypesMapping.BuildAgentPoolProviderSpecMappings,
                DerivedTypesMapping.ActivityEventInfoMappings
            ],
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

        converters.Add(new JsonStringEnumConverter<ResourceType>());
        converters.Add(new JsonStringEnumConverter<PermissionLevel>());
        converters.Add(new JsonStringEnumConverter<SpecificPermission>());
    }

    internal static WebApplicationBuilder AddCitadelRateLimiter(this WebApplicationBuilder builder)
    {
        builder.Services.AddRateLimiter(options =>
        {
            options.GlobalLimiter = PartitionedRateLimiter.Create<HttpContext, string>(context =>
            {
                var actorId = context.User.Identity?.IsAuthenticated == true ? context.User?.GetActorId() : null;
                var ip = context.Connection.RemoteIpAddress?.ToString();

                var key = actorId is not null
                    ? $"actor:{actorId}"
                    : ip is not null ? $"ip:{ip}" : "ip:unknown";

                return RateLimitPartition.GetSlidingWindowLimiter(
                    partitionKey: key,
                    factory: _ => new SlidingWindowRateLimiterOptions
                    {
                        PermitLimit = 120,
                        Window = TimeSpan.FromMinutes(1),
                        SegmentsPerWindow = 6,
                        QueueLimit = 10,
                        AutoReplenishment = true
                    });
            });

            options.AddPolicy("strict-auth", context =>
            {
                var ip = context.Connection.RemoteIpAddress?.ToString() ?? "unknown";

                return RateLimitPartition.GetFixedWindowLimiter(
                    partitionKey: $"auth:{ip}",
                    factory: _ => new FixedWindowRateLimiterOptions
                    {
                        PermitLimit = 6,
                        Window = TimeSpan.FromSeconds(30),
                        QueueLimit = 0,
                        AutoReplenishment = true
                    });
            });

            options.AddPolicy("webhook-listener", context =>
            {
                var ip = context.Connection.RemoteIpAddress?.ToString() ?? "unknown";

                return RateLimitPartition.GetSlidingWindowLimiter(
                    partitionKey: $"webhook:{ip}",
                    factory: _ => new SlidingWindowRateLimiterOptions
                    {
                        PermitLimit = 120,
                        Window = TimeSpan.FromMinutes(1),
                        SegmentsPerWindow = 6,
                        QueueLimit = 20,
                        AutoReplenishment = true
                    });
            });

            options.OnRejected = async (context, token) =>
            {
                var retryAfter = context.Lease.TryGetMetadata(MetadataName.RetryAfter, out var retry)
                    ? retry.TotalSeconds
                    : (double?)null;

                if (retryAfter is not null)
                    context.HttpContext.Response.Headers.RetryAfter = ((int)retryAfter.Value).ToString();

                var details = retryAfter is not null
                    ? $"Rate limit exceeded. Try again in {Math.Ceiling(retryAfter.Value)} seconds."
                    : "Rate limit exceeded. Try again later.";

                await Helpers.WriteResponse(
                    context.HttpContext, 
                    new Exception(details), 
                    StatusCodes.Status429TooManyRequests, 
                    "Too many requests");
            };
        });

        return builder;
    }

    internal static void AddIOptionsFromConfiguration(this WebApplicationBuilder builder)
    {
        builder.Services.AddOptions<JwtConfiguration>().BindConfiguration("Jwt").ValidateOnStart();

        var secretsOptions = builder.Services
            .AddOptions<SecretsConfiguration>()
            .BindConfiguration(SecretsConfiguration.SectionName);
        if (!Helpers.IsDesignTime())
        {
            secretsOptions
                .Validate(
                    options => SecretsConfiguration.IsValidEncryptionKey(options.EncryptionKey),
                    "Secrets:EncryptionKey must be a base64-encoded 32-byte key.")
                .ValidateOnStart();
        }

        builder.Services.AddOptions<JobConfiguration>().BindConfiguration("JobConfiguration").ValidateOnStart();
        builder.Services
            .AddOptions<EdgeAgentOptions>()
            .BindConfiguration(EdgeAgentOptions.SectionName)
            .Validate(options => !string.IsNullOrWhiteSpace(options.AgentImageRepository), "EdgeAgent:AgentImageRepository is required.")
            .ValidateOnStart();
        builder.Services
            .AddOptions<MfaOptions>()
            .BindConfiguration(MfaOptions.SectionName)
            .Validate(options => options.ChallengeLifetimeMinutes is > 0 and <= 5, "Mfa:ChallengeLifetimeMinutes must be between 1 and 5.")
            .Validate(options => options.SetupLifetimeMinutes is > 0 and <= 10, "Mfa:SetupLifetimeMinutes must be between 1 and 10.")
            .Validate(options => options.MaxFailedAttempts > 0, "Mfa:MaxFailedAttempts must be greater than zero.")
            .Validate(options => options.RecoveryCodeCount > 0, "Mfa:RecoveryCodeCount must be greater than zero.")
            .ValidateOnStart();
        builder.Services
            .AddOptions<BootstrapOptions>()
            .BindConfiguration(BootstrapOptions.SectionName);
        builder.Services
            .AddOptions<AutomationOptions>()
            .BindConfiguration(AutomationOptions.SectionName)
            .Validate(options => options.MaxParallelRuns > 0, "Automations:MaxParallelRuns must be greater than zero.")
            .Validate(options => options.DefaultTimeoutSeconds > 0, "Automations:DefaultTimeoutSeconds must be greater than zero.")
            .Validate(options => options.MaxTimeoutSeconds >= options.DefaultTimeoutSeconds, "Automations:MaxTimeoutSeconds must be greater than or equal to Automations:DefaultTimeoutSeconds.")
            .Validate(options => options.MaxLogBytes >= 4096, "Automations:MaxLogBytes must be at least 4096 bytes.")
            .Validate(IsValidAutomationInternalBaseUrl, "Automations:InternalBaseUrl must be an absolute HTTP or HTTPS URL.")
            .ValidateOnStart();
        builder.Services
            .AddOptions<BuildOptions>()
            .BindConfiguration(BuildOptions.SectionName)
            .Validate(options => options.MaxParallelRuns > 0, "Builds:MaxParallelRuns must be greater than zero.")
            .Validate(options => options.RunRetentionDays > 0, "Builds:RunRetentionDays must be greater than zero.")
            .ValidateOnStart();
        builder.Services
            .AddOptions<BackupOptions>()
            .BindConfiguration(BackupOptions.SectionName)
            .Validate(options => !string.IsNullOrWhiteSpace(options.ResticPath), "Backups:ResticPath is required.")
            .Validate(options => !string.IsNullOrWhiteSpace(options.PostgresDumpPath), "Backups:PostgresDumpPath is required.")
            .Validate(options => !string.IsNullOrWhiteSpace(options.WorkingDirectory), "Backups:WorkingDirectory is required.")
            .Validate(options => options.RepositoryLeaseSeconds >= 30, "Backups:RepositoryLeaseSeconds must be at least 30 seconds.")
            .Validate(options => options.DefaultTimeoutSeconds > 0, "Backups:DefaultTimeoutSeconds must be greater than zero.")
            .Validate(options => options.MaxLogLineBytes >= 1024, "Backups:MaxLogLineBytes must be at least 1024 bytes.")
            .ValidateOnStart();
    }

    private static bool IsValidAutomationInternalBaseUrl(AutomationOptions options)
        => Uri.TryCreate(options.InternalBaseUrl, UriKind.Absolute, out var uri)
           && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps)
           && !string.IsNullOrWhiteSpace(uri.Host);
}

// Required for integration tests to work properly
public partial class Program { }
