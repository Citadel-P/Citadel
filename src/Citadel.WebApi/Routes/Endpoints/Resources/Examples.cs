using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;
using Application.Models;
using Domain;
using Domain.Entities.Registries;
using Microsoft.OpenApi;
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
                var registry = new RegistryInput(Name: "my-azure-registry", Url: "myproject.azurecr.io", Type: RegistryType.Azure, AzureRegistry.Create("johnDoe", "myPassword"));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample CreateAwsRegistryExample()
            {
                var registry = new RegistryInput(Name: "my-ecr-registry", Url: "aws-account-id.dkr.ecr.us-east-2.amazonaws.com/", Type: RegistryType.AWS,
                    AWSRegistry.Create(true, "", "", "us-east-2"));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample CreateGitlabRegistryExample()
            {
                var registry = new RegistryInput(Name: "", Url: "https://registry.gitlab.com", Type: RegistryType.Gitlab,
                    GitlabRegistry.Create("", "", "https://gitlab.com"));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample CreateDockerHubRegistryExample()
            {
                var registry = new RegistryInput(Name: "my-custom-registry", Url: "", Type: RegistryType.DockerHub,
                    DockerHubRegistry.Create("", ""));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample CreateGitHubRegistryExample()
            {
                var registry = new RegistryInput(Name: "my-custom-registry", Url: "", Type: RegistryType.GitHub,
                    GitHubRegistry.Create("organization or user name", "", GhcrAccountType.Organization));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
        }
        internal static class Update
        {
            internal static OpenApiExample UpdateAzureRegistryExample()
            {
                var registry = new RegistryInput(Name: "my-azure-registry", Url: "myproject.azurecr.io", Type: RegistryType.Azure, AzureRegistry.Create("johnDoe", "myPassword"));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample UpdateAwsRegistryExample()
            {
                var registry = new RegistryInput(Name: "my-ecr-registry", Url: "aws-account-id.dkr.ecr.us-east-2.amazonaws.com/", Type: RegistryType.AWS,
                    AWSRegistry.Create(true, "", "", "us-east-2"));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample UpdateGitlabRegistryExample()
            {
                var registry = new RegistryInput(Name: "", Url: "https://registry.gitlab.com", Type: RegistryType.Gitlab,
                    GitlabRegistry.Create("", "", "https://gitlab.com"));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample UpdateDockerHubRegistryExample()
            {
                var registry = new RegistryInput(Name: "my-custom-registry", Url: "", Type: RegistryType.DockerHub,
                    DockerHubRegistry.Create("", ""));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
            internal static OpenApiExample UpdateGitHubRegistryExample()
            {
                var registry = new RegistryInput(Name: "my-custom-registry", Url: "", Type: RegistryType.GitHub,
                    GitHubRegistry.Create("organization or user name", "", GhcrAccountType.Organization));
                return new OpenApiExample() { Value = JsonNode.Parse(JsonSerializer.Serialize(registry, typeof(RegistryInput), GetJsonContext())) };
            }
        }

        internal static ApplicationJsonContext GetJsonContext()
        {
            var options = new JsonSerializerOptions()
            {
                Converters = { new JsonStringEnumConverter() },
                DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
            };
            return new ApplicationJsonContext(options);
        }
    }
    
}
