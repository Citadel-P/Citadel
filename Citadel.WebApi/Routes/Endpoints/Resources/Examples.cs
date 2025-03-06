using Infrastructure.Entities;
using Infrastructure;
using Microsoft.OpenApi.Any;
using Microsoft.OpenApi.Models;
using System.Text.Json;
using WebApi.Routes.Endpoints.Resources.Registries;
using System.Text.Json.Serialization;
using Hosting.Converters;
using Microsoft.Win32;

namespace WebApi.Routes.Endpoints.Resources;

internal static class Examples
{

    internal static class Registries
    {
        internal static OpenApiExample CreateAzureRegistryExample()
        {
            var registry = new CreateRegistryInput(Name: "my-azure-registry", Url: "myproject.azurecr.io", Discriminator: RegistryDiscriminator.Azure, AzureRegistry.Create("johnDoe", "myPassword"));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, jsonOptions)) };
        }
        internal static OpenApiExample CreateAwsRegistryExample()
        {
            var registry = new CreateRegistryInput(Name: "my-ecr-registry", Url: "aws-account-id.dkr.ecr.us-east-2.amazonaws.com/", Discriminator: RegistryDiscriminator.AWS,
                AWSRegistry.Create(true, "", "", "us-east-2"));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, jsonOptions)) };
        }
        internal static OpenApiExample CreateGitlabRegistryExample()
        {
            var registry = new CreateRegistryInput(Name: "", Url: "https://registry.gitlab.com", Discriminator: RegistryDiscriminator.Gitlab,
                GitlabRegistry.Create("", "", "https://gitlab.com"));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, jsonOptions)) };
        }
        internal static OpenApiExample CreateDockerHubRegistryExample()
        {
            var registry = new CreateRegistryInput(Name: "my-custom-registry", Url: "", Discriminator: RegistryDiscriminator.DockerHub,
                DockerHubRegistry.Create("", ""));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, jsonOptions)) };
        }
        internal static OpenApiExample CreateGitHubRegistryExample()
        {
            var registry = new CreateRegistryInput(Name: "my-custom-registry", Url: "", Discriminator: RegistryDiscriminator.GitHub,
                GitHubRegistry.Create("organization or user name", "", GhcrAccountType.Organization));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, jsonOptions)) };
        }

        private static readonly JsonSerializerOptions jsonOptions = DefaultJsonOptions();
        private static JsonSerializerOptions DefaultJsonOptions()
        {
            var jsonOptions = new JsonSerializerOptions();
            jsonOptions.Converters.Add(new JsonStringEnumConverter());
            jsonOptions.Converters.Add(new DatetimeOffsetConverter());
            jsonOptions.Converters.Add(new DatetimeConverter());
            return jsonOptions;
        }
    }
    
}
