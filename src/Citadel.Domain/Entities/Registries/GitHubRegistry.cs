using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record GitHubRegistry(string NameSpace, bool? GhcrAuthEnabled = false, string? PAT = null) : RegistryConfigurationBase
{
    public override string GetRegistryAuth(string registryHost) => new RegistryAuth(NameSpace, PAT ?? string.Empty, registryHost).GetAuth();
   
    public static GitHubRegistry Create(string nameSpace, bool? ghcrAuthEnabled, string? PAT) => new (nameSpace, ghcrAuthEnabled, PAT);
}

