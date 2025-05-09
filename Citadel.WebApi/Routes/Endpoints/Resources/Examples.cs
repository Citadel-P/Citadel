using System.Text.Json;
using System.Text.Json.Serialization;
using Infrastructure;
using Infrastructure.Entities;
using Microsoft.OpenApi.Any;
using Microsoft.OpenApi.Models;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes.Endpoints.Resources;

internal static class Examples
{

    internal static class Registries
    {
        internal static class Create
        {
            internal static OpenApiExample CreateAzureRegistryExample()
            {
                var registry = new CreateRegistryInput(Name: "my-azure-registry", Url: "myproject.azurecr.io", Discriminator: RegistryDiscriminator.Azure, AzureRegistry.Create("johnDoe", "myPassword"));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(CreateRegistryInput) , RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample CreateAwsRegistryExample()
            {
                var registry = new CreateRegistryInput(Name: "my-ecr-registry", Url: "aws-account-id.dkr.ecr.us-east-2.amazonaws.com/", Discriminator: RegistryDiscriminator.AWS,
                    AWSRegistry.Create(true, "", "", "us-east-2"));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(CreateRegistryInput), RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample CreateGitlabRegistryExample()
            {
                var registry = new CreateRegistryInput(Name: "", Url: "https://registry.gitlab.com", Discriminator: RegistryDiscriminator.Gitlab,
                    GitlabRegistry.Create("", "", "https://gitlab.com"));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(CreateRegistryInput), RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample CreateDockerHubRegistryExample()
            {
                var registry = new CreateRegistryInput(Name: "my-custom-registry", Url: "", Discriminator: RegistryDiscriminator.DockerHub,
                    DockerHubRegistry.Create("", ""));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(CreateRegistryInput), RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample CreateGitHubRegistryExample()
            {
                var registry = new CreateRegistryInput(Name: "my-custom-registry", Url: "", Discriminator: RegistryDiscriminator.GitHub,
                    GitHubRegistry.Create("organization or user name", "", GhcrAccountType.Organization));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(CreateRegistryInput), RegistryExampleContext.Default)) };
            }
        }
        internal static class Update
        {
            internal static OpenApiExample UpdateAzureRegistryExample()
            {
                var registry = new PatchRegistryInput(Id: Guid.Empty, Name: "my-azure-registry", Url: "myproject.azurecr.io", Discriminator: RegistryDiscriminator.Azure, AzureRegistry.Create("johnDoe", "myPassword"));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(PatchRegistryInput), RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample UpdateAwsRegistryExample()
            {
                var registry = new PatchRegistryInput(Id: Guid.Empty, Name: "my-ecr-registry", Url: "aws-account-id.dkr.ecr.us-east-2.amazonaws.com/", Discriminator: RegistryDiscriminator.AWS,
                    AWSRegistry.Create(true, "", "", "us-east-2"));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(PatchRegistryInput), RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample UpdateGitlabRegistryExample()
            {
                var registry = new PatchRegistryInput(Id: Guid.Empty, Name: "", Url: "https://registry.gitlab.com", Discriminator: RegistryDiscriminator.Gitlab,
                    GitlabRegistry.Create("", "", "https://gitlab.com"));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(PatchRegistryInput), RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample UpdateDockerHubRegistryExample()
            {
                var registry = new PatchRegistryInput(Id: Guid.Empty, Name: "my-custom-registry", Url: "", Discriminator: RegistryDiscriminator.DockerHub,
                    DockerHubRegistry.Create("", ""));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(PatchRegistryInput), RegistryExampleContext.Default)) };
            }
            internal static OpenApiExample UpdateGitHubRegistryExample()
            {
                var registry = new PatchRegistryInput(Id: Guid.Empty, Name: "my-custom-registry", Url: "", Discriminator: RegistryDiscriminator.GitHub,
                    GitHubRegistry.Create("organization or user name", "", GhcrAccountType.Organization));
                return new OpenApiExample() { Value = new OpenApiString(JsonSerializer.Serialize(registry, typeof(PatchRegistryInput), RegistryExampleContext.Default)) };
            }
        }

        
    }
    
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(CreateRegistryInput))]
[JsonSerializable(typeof(PatchRegistryInput))]
[JsonSerializable(typeof(RegistryConfigurationBase))]
internal partial class RegistryExampleContext : JsonSerializerContext
{
}
