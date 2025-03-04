using Infrastructure.Entities;
using Infrastructure;
using Microsoft.OpenApi.Any;
using Microsoft.OpenApi.Models;
using System.Text.Json;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes.Endpoints.Resources;

internal static class Examples
{

    internal static class Registries
    {
        internal static OpenApiExample CreateAzureRegistryExample()
        {
            var azureRegistry = new CreateRegistryRequest(Name: "my-azure-registry", Url: "myproject.azurecr.io", Discriminator: RegistryDiscriminator.Azure, AzureRegistry.Create("johnDoe", "myPassword"));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(azureRegistry)) };
        }
        internal static OpenApiExample CreateAwsRegistryExample()
        {
            var awsRegistry = new CreateRegistryRequest(Name: "my-ecr-registry", Url: "aws-account-id.dkr.ecr.us-east-2.amazonaws.com/", Discriminator: RegistryDiscriminator.AWS,
                AWSRegistry.Create(true, "", "", "us-east-2"));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(awsRegistry)) };
        }

        internal static OpenApiExample CreateGitlabRegistryExample()
        {
            var gitlabRegistry = new CreateRegistryRequest(Name: "", Url: "https://registry.gitlab.com", Discriminator: RegistryDiscriminator.Gitlab,
                GitlabRegistry.Create("", "", "https://gitlab.com"));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(gitlabRegistry)) };
        }
        internal static OpenApiExample CreateCustomRegistryExample()
        {
            var customRegistry = new CreateRegistryRequest(Name: "my-custom-registry", Url: "10.0.0.1 or myregistry.domain.tld", Discriminator: RegistryDiscriminator.Custom,
                CustomRegistry.Create(true, "", ""));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(customRegistry)) };
        }
        internal static OpenApiExample CreateDockerHubRegistryExample()
        {
            var dockerRegistry = new CreateRegistryRequest(Name: "my-custom-registry", Url: "", Discriminator: RegistryDiscriminator.DockerHub,
                DockerHubRegistry.Create("", ""));
            return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(dockerRegistry)) };
        }
    }
    
}
