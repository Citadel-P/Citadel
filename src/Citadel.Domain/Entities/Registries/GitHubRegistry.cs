using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record GitHubRegistry(bool? GhcrAuthEnabled = false, string? Name = null, string? PAT = null, GhcrAccountType? AccountType = null) : RegistryConfigurationBase
{
    public override string GetRegistryAuth(string registryHost) => new RegistryAuth(Name ?? string.Empty, PAT ?? string.Empty, registryHost).GetAuth();
   
    public static GitHubRegistry Create(bool? ghcrAuthEnabled, string? name, string? PAT, GhcrAccountType? accountType) => new (ghcrAuthEnabled, name, PAT, accountType);
}

