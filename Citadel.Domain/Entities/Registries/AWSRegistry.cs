using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public record AWSRegistry(string AccessKey, bool AuthenticationRequired, string SecretAccessKey, string Region) : RegistryConfigurationBase
{
    public override string RegistryUrl => string.Empty;
    
    public static AWSRegistry Create(bool authenticationRequired, string accessKey, string secretAccessKey, string region)
        => new (accessKey, authenticationRequired, secretAccessKey, region);

    public override string GetRegistryAuth()
    {
        throw new NotImplementedException();
    }
}

