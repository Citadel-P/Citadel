using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record GitHubRegistry(string Name, string PAT, GhcrAccountType? Type) : RegistryConfigurationBase
{
    public override string RegistryUrl => "https://ghcr.io";

    public static GitHubRegistry Create(string name, string PAT, GhcrAccountType type) =>
        new (name, PAT, type);

    public override string GetRegistryAuth() => new RegistryAuth(Name, PAT, RegistryUrl).GetAuth();
}

