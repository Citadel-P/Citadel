using Hosting;
using WebApi;
using Application;
using Infrastructure;
using WebApi.Helpers;
using Application.Models;
using Microsoft.AspNetCore.Http.Json;
using Hosting.OpenApi;
using Application.Configs;
using Infrastructure.TaskJobs;

CitadelWebApplicationBuilder.Create(args, new CitadelWebApplicationOptions()
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
        .RegisterInfrastructureModule(builder.Configuration)
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
    services.Configure<JobConfiguration>(configuration.GetSection("JobConfiguration"));
    return services;
}
