using Application;
using Application.Configs;
using Application.Models;
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
    WithJsonOptions = AdditionalJsonOptions
});

// Add services to the container.
void WithServices(WebApplicationBuilder builder)
{
    builder.Services
        .RegisterInfrastructureModule(builder.Configuration)
        .RegisterWebApiModule(builder.Configuration)
        .RegisterApplicationModule()
        .AddHealthChecks();

    AddIOptionsFromConfiguration(builder);
}

// Configures the HTTP request pipeline.
async void Configure(WebApplication app)
{
    // var pubKey = Helpers.GetOrCreatePublicKey();
    if (app.Configuration.GetValue<bool>("EnableSwagger"))
    {
        app.MapOpenApi();
        app.UseSwaggerUI(options => 
        {
            options
            .AddCustomSwaggerUIOptions()
            .SwaggerEndpoint("/openapi/v1.json", "v1");
        });
    }

    app.UseWebApiModule();
    app.MapHealthChecks("/health", HealthCheck.GetHealthCheckOptions());
}

void AdditionalJsonOptions(JsonOptions options)
{
    options.SerializerOptions.TypeInfoResolverChain.Add(ApplicationJsonContext.Default);
    //options.SerializerOptions.TypeInfoResolverChain.Add(DeploymentJsonContext.Default);
    //options.SerializerOptions.TypeInfoResolverChain.Add(RegistryJsonContext.Default);
    options.SerializerOptions.TypeInfoResolverChain.Add(ProblemJsonContext.Default);
    options.SerializerOptions.Converters.AddGenericEnumConverters();
    Citadel.GeneratedConverters.SafeEnumConverters.Register(options.SerializerOptions);
}

static void AddIOptionsFromConfiguration(WebApplicationBuilder builder)
{
    builder.Services.AddOptions<JwtConfiguration>().BindConfiguration("Jwt").ValidateOnStart();
    builder.Services.AddOptions<JobConfiguration>().BindConfiguration("JobConfiguration").ValidateOnStart();
}
