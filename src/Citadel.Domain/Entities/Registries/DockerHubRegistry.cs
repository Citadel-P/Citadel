using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record DockerHubRegistry(string? UserName = null, string? PAT = null) : RegistryConfigurationBase
{
    public override string? GetRegistryAuth(string registryHost) => new RegistryAuth(UserName ?? string.Empty, PAT ?? string.Empty, registryHost).GetAuth();
    
    public static DockerHubRegistry Create(string userName, string PAT) => new (userName, PAT);
}

