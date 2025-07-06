using Application;
using Application.Configs;
using Application.Models;
using Domain;
using Hosting;
using Hosting.Common;
using Hosting.OpenApi;
using Infrastructure;
using Microsoft.AspNetCore.Http.Json;
using WebApi;

await CitadelWebApplicationBuilder.Create(args, new CitadelWebApplicationOptions()
{
    Configure = Configure,
    WithServices = WithServices,
    WithJsonResponseOptions = AdditionalJsonResponseOptions
});

// Add services to the container.
void WithServices(WebApplicationBuilder builder)
{
    builder.Services
        .RegisterWebApiModule(builder.Configuration)
        .RegisterApplicationModule()
        .RegisterInfrastructureModule(builder.Environment)
        .AddHealthChecks();

    AddIOptionsFromConfiguration(builder.Services, builder.Configuration);
}

// Configures the HTTP request pipeline.
void Configure(WebApplication app)
{
    if (app.Environment.IsDevelopment())
    {
        app.MapOpenApi();
        app.UseSwaggerUI(options => 
        {
            options.AddCustomSwaggerUIOptions(app.Environment.IsDevelopment());
            options.SwaggerEndpoint("/openapi/v1.json", "v1");
        });
    }

    app.UseWebApiModule();
    app.MapHealthChecks("/health", HealthCheck.GetHealthCheckOptions());
}

void AdditionalJsonResponseOptions(JsonOptions options)
{
    options.SerializerOptions.TypeInfoResolverChain.Add(ApplicationJsonContext.Default);
    options.SerializerOptions.TypeInfoResolverChain.Add(ProblemJsonContext.Default);
    options.SerializerOptions.TypeInfoResolverChain.Add(RegistryJsonContext.Default);
    options.SerializerOptions.Converters.AddGenericEnumConverters();
    Citadel.GeneratedConverters.SafeEnumConverters.Register(options.SerializerOptions);
}

static IServiceCollection AddIOptionsFromConfiguration(IServiceCollection services, IConfiguration configuration)
{
    services.Configure<JwtConfiguration>(configuration.GetSection("Jwt"));
    services.Configure<JobConfiguration>(configuration.GetSection("JobConfiguration"));
    return services;
}
