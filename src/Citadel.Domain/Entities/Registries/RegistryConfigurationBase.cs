using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(AWSRegistry), nameof(RegistryType.AWS))]
[JsonDerivedType(typeof(AzureRegistry), nameof(RegistryType.Azure))]
[JsonDerivedType(typeof(GitlabRegistry), nameof(RegistryType.Gitlab))]
[JsonDerivedType(typeof(DockerHubRegistry), nameof(RegistryType.DockerHub))]
[JsonDerivedType(typeof(GitHubRegistry), nameof(RegistryType.GitHub))]
[JsonDerivedType(typeof(CustomRegistry), nameof(RegistryType.Custom))]
public abstract record RegistryConfigurationBase
{
    public abstract string? GetRegistryAuth(string registryHost);
}

