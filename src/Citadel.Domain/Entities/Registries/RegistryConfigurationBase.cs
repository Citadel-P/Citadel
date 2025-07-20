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
public record RegistryConfigurationBase(string RegistryUrl)
{
    public virtual string GetRegistryAuth() 
    {
        return string.Empty;
    }
}

