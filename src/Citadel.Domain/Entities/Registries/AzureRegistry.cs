using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record AzureRegistry(string UserName, string Password) : RegistryConfigurationBase
{
    public static AzureRegistry Create(string userName, string password) => 
        new (userName, password);
    public override string GetRegistryAuth(string registryHost) => throw new NotImplementedException();
}

