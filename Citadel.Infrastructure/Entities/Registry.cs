using System.Text.Json;
using System.Text.Json.Serialization;

namespace Infrastructure.Entities;

/// <summary>
/// Represent a container registry
/// </summary>
public class Registry
{
    public Guid Id { get; private set; }
    public string Name { get; private set; }
    public string Url { get; private set; }
    public DateTime Created { get; private set; }

    /// <summary>
    /// The name of the type that will be serialized/deserialized
    /// </summary>
    public RegistryDiscriminator Discriminator { get; private set; }

    /// <summary>
    /// The registry configuration serialized as a json string.
    /// Contains an implementation of the <see cref="IRegistryConfiguration"/>
    /// </summary>
    public string Configuration { get; set; }

    public static Registry Create(string name, string url, RegistryDiscriminator discriminator, IRegistryConfiguration configuration) 
        => new ()
        {
            Id = Guid.CreateVersion7(),
            Created = DateTime.UtcNow,
            Name = name,
            Url = url,
            Discriminator = discriminator,
            Configuration = SerializeConfiguration(configuration, discriminator)
        };

    private static string SerializeConfiguration(IRegistryConfiguration configuration, RegistryDiscriminator discriminator)
    {
        string serializedCfg = string.Empty;
        switch (discriminator)
        {
            case RegistryDiscriminator.Azure:
                serializedCfg = JsonSerializer.Serialize((AzureRegistry)configuration);
                break;

            case RegistryDiscriminator.AWS:
                serializedCfg = JsonSerializer.Serialize((AWSRegistry)configuration);
                break;

            case RegistryDiscriminator.DockerHub:
                serializedCfg = JsonSerializer.Serialize((DockerHubRegistry)configuration);
                break;

            case RegistryDiscriminator.Gitlab:
                serializedCfg = JsonSerializer.Serialize((GitlabRegistry)configuration);
                break;

            case RegistryDiscriminator.Custom:
                serializedCfg = JsonSerializer.Serialize((CustomRegistry)configuration);
                break;
        }
        return serializedCfg;
    }
}

[JsonPolymorphic]
[JsonDerivedType(typeof(AWSRegistry), (int)RegistryDiscriminator.AWS)]
[JsonDerivedType(typeof(AzureRegistry), (int)RegistryDiscriminator.Azure)]
[JsonDerivedType(typeof(GitlabRegistry), (int)RegistryDiscriminator.Gitlab)]
[JsonDerivedType(typeof(CustomRegistry), (int)RegistryDiscriminator.Custom)]
[JsonDerivedType(typeof(DockerHubRegistry), (int)RegistryDiscriminator.DockerHub)]
public interface IRegistryConfiguration
{ }

public class DockerHubRegistry : IRegistryConfiguration
{
    [JsonInclude]
    public string UserName { get; private set; }
    [JsonInclude]
    public string PAT { get; private set; }
    public static DockerHubRegistry Create(string userName, string PAT) 
        => new()
        {
            UserName = userName,
            PAT = PAT
        };
}

public class AzureRegistry : IRegistryConfiguration
{
    [JsonInclude]
    public string UserName { get; private set; }
    [JsonInclude]
    public string Password { get; private set; }

    public static AzureRegistry Create(string userName, string password) => 
        new () 
        {
            Password = password,
            UserName = userName 
        };
}

public class AWSRegistry : IRegistryConfiguration
{
    /// <summary>
    /// If true, the credential bellow should be specified in order to connect to a private AWS registry
    /// </summary>
    [JsonInclude]
    public bool AuthenticationRequired { get; private set; }
    [JsonInclude]
    public string AccessKey { get; private set; }
    [JsonInclude]
    public string SecretAccessKey { get; private set; }
    [JsonInclude]
    public string Region { get; private set; }

    public static AWSRegistry Create(bool authenticationRequired, string accessKey, string secretAccessKey, string region)
        => new()
        {
            AuthenticationRequired = authenticationRequired,
            AccessKey = accessKey,
            SecretAccessKey = secretAccessKey,
            Region = region
        };
}

public class GitlabRegistry : IRegistryConfiguration
{
    [JsonInclude]
    public string UserName { get; private set; }
    [JsonInclude]
    public string PAT { get; private set; }
    [JsonInclude]
    public string InstanceUrl { get; private set; }

    public static GitlabRegistry Create(string userName, string PAT, string instanceUrl) 
        => new ()
        {
            UserName = userName,
            PAT = PAT,
            InstanceUrl = instanceUrl
        };
}

public class CustomRegistry : IRegistryConfiguration
{
    /// <summary>
    /// If true, the credential bellow should be specified in order to connect to the custom registry
    /// </summary>
    [JsonInclude]
    public bool AuthenticationRequired { get; set; }

    [JsonInclude]
    public string UserName { get; private set; }
    [JsonInclude]
    public string Password { get; private set; }
    public static CustomRegistry Create(bool authenticationRequired, string userName, string password)
        => new()
        {
            AuthenticationRequired = authenticationRequired,
            UserName = userName,
            Password = password
        };
}