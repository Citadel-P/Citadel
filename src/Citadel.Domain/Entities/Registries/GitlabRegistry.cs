using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record GitlabRegistry(string UserName, string PAT, string InstanceUrl) : RegistryConfigurationBase
{
    public static GitlabRegistry Create(string userName, string PAT, string instanceUrl) 
        => new (userName, PAT, instanceUrl);

    public override string GetRegistryAuth(string registryUrl)
    {
        throw new NotImplementedException();
    }
}

