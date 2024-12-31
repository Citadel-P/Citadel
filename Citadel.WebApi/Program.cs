using Hosting;
using WebApi;
using Application;
using Infrastructure;
using WebApi.Helpers;
using Microsoft.OpenApi.Models;
using Swashbuckle.AspNetCore.SwaggerGen;
using Microsoft.AspNetCore.Mvc;
using Application.Models;
using Common.Configs;

DTWebApplicationBuilder.Create(args, new DTWebApplicationOptions()
{
    Configure = Configure,
    WithServices = WithServices,
    WithAdditionalSwaggerOptions = AdditionalSwaggerOptions,
    WithAdditionalJsonOptions = AdditionalJsonOptions
});

// Add services to the container.
void WithServices(WebApplicationBuilder builder)
{
    builder.Services
        .RegisterWebApiModule(builder.Configuration)
        .RegisterApplicationModule()
        .RegisterInfrastructureModule()
        .AddHealthChecks();

    AddIOptionsFromConfiguration(builder.Services, builder.Configuration);
}

// Configures the HTTP request pipeline.
void Configure(WebApplication app)
{
    app
        .UseWebApiModule();

    app.MapFallbackToFile("index.html");
    app.MapHealthChecks("/health", HealthCheckOptionsHelper.GetHealthCheckOptions());
}

void AdditionalSwaggerOptions(SwaggerGenOptions options)
{
    options
        .AddSecurityDefinition("Bearer", new OpenApiSecurityScheme
        {
            Description = "JWT Authorization header using the Bearer scheme. Example: \"{token}\"",
            Name = "Authorization",
            In = ParameterLocation.Header,
            Scheme = "bearer",
            Type = SecuritySchemeType.Http,
            BearerFormat = "JWT"
        });

    options
        .AddSecurityRequirement(new OpenApiSecurityRequirement
        {
            {
                new OpenApiSecurityScheme
                {
                    Reference = new OpenApiReference { Type = ReferenceType.SecurityScheme, Id = "Bearer" }
                },
                new List<string>()
            }
        });
}

void AdditionalJsonOptions(JsonOptions options)
{
    options.JsonSerializerOptions.TypeInfoResolverChain.Add(ApplicationJsonContext.Default);
    options.JsonSerializerOptions.TypeInfoResolverChain.Add(ProblemJsonContext.Default);
}

static IServiceCollection AddIOptionsFromConfiguration(IServiceCollection services, IConfiguration configuration)
{
    services.Configure<JwtConfig>(configuration.GetSection("Jwt"));
    return services;
}