using Hosting;
using WebApi;
using Application;
using Infrastructure;
using WebApi.Helpers;
using Application.Models;
using Common.Configs;
using Microsoft.AspNetCore.Http.Json;
using WebApi.Swagger;

DTWebApplicationBuilder.Create(args, new DTWebApplicationOptions()
{
    Configure = Configure,
    WithServices = WithServices,
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
    if (app.Environment.IsDevelopment())
    {
        app.MapOpenApi();
        app.UseSwaggerUI(options => options.AddCustomSwaggerUIOptions(app.Environment.IsDevelopment()));
    }

    app.UseWebApiModule();

    app.MapFallbackToFile("index.html");
    app.MapHealthChecks("/health", HealthCheckOptionsHelper.GetHealthCheckOptions());
}

void AdditionalJsonOptions(JsonOptions options)
{
    options.SerializerOptions.TypeInfoResolverChain.Add(ApplicationJsonContext.Default);
    options.SerializerOptions.TypeInfoResolverChain.Add(ProblemJsonContext.Default);
}

static IServiceCollection AddIOptionsFromConfiguration(IServiceCollection services, IConfiguration configuration)
{
    services.Configure<JwtConfig>(configuration.GetSection("Jwt"));
    return services;
}