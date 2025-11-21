using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record CustomRegistry(bool? AuthEnabled = false, string? UserName = null, string? Password = null) : RegistryConfigurationBase
{
    public static CustomRegistry Create(bool? authEnabled, string? userName, string? PAT) => new (authEnabled, userName, PAT);
    public override string GetRegistryAuth(string registryUrl) => new RegistryAuth(UserName ?? string.Empty, Password ?? string.Empty, registryUrl).GetAuth();
}

