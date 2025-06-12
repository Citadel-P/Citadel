using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record AzureRegistry(string UserName, string Password) : RegistryConfigurationBase
{
    public override string RegistryUrl => string.Empty;
    
    public static AzureRegistry Create(string userName, string password) => 
        new (userName, password);

    public override string GetRegistryAuth()
    {
        throw new NotImplementedException();
    }
}

