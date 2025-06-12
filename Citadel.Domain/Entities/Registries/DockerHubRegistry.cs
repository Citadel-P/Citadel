using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record DockerHubRegistry(string? UserName = null, string? PAT = null) : RegistryConfigurationBase
{
    public override string RegistryUrl => "https://docker.io";
    public static DockerHubRegistry Create(string userName, string PAT) => new (userName, PAT);
    public override string GetRegistryAuth() => new RegistryAuth(UserName, PAT, RegistryUrl).GetAuth();
}

