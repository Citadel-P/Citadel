using System.Text.Json.Serialization;

namespace Infrastructure.Entities.Registries;

[method: JsonConstructor]
public record GitlabRegistry(string UserName, string PAT, string InstanceUrl) : RegistryConfigurationBase
{
    public override string RegistryUrl => string.Empty;
    public static GitlabRegistry Create(string userName, string PAT, string instanceUrl) 
        => new (userName, PAT, instanceUrl);

    public override string GetRegistryAuth()
    {
        throw new NotImplementedException();
    }
}

