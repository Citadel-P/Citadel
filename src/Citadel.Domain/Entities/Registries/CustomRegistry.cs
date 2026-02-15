using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record CustomRegistry(bool? AuthEnabled = false, string? UserName = null, string? Password = null) : RegistryConfigurationBase
{
    public override string? GetRegistryAuth(string registryHost) => AuthEnabled == true
        ? new RegistryAuth(UserName ?? string.Empty, Password ?? string.Empty, registryHost).GetAuth()
        : null;
    
    public static CustomRegistry Create(bool? authEnabled, string? userName, string? PAT) => new (authEnabled, userName, PAT);
}

