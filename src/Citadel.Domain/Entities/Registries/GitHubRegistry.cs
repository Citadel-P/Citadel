using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record GitHubRegistry(string Name, string PAT, GhcrAccountType AccountType) : RegistryConfigurationBase
{
    public override string GetRegistryAuth(string registryHost) => new RegistryAuth(Name, PAT, registryHost).GetAuth();
   
    public static GitHubRegistry Create(string name, string PAT, GhcrAccountType accountType) => new (name, PAT, accountType);
}

