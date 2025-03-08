using System.Text.Json;
using System.Text.Json.Serialization;
using Hosting.Common;

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
            Configuration = JsonSerializer.Serialize(configuration, Helpers.CommonJsonOptions)
        };

    private static string SerializeConfiguration(IRegistryConfiguration configuration, RegistryDiscriminator discriminator)
    {
        string serializedCfg = string.Empty;
        switch (discriminator)
        {
            case RegistryDiscriminator.Azure:
                serializedCfg = JsonSerializer.Serialize(configuration, Helpers.CommonJsonOptions);
                break;
            case RegistryDiscriminator.AWS:
                serializedCfg = JsonSerializer.Serialize(configuration, Helpers.CommonJsonOptions);
                break;
            case RegistryDiscriminator.DockerHub:
                serializedCfg = JsonSerializer.Serialize(configuration, Helpers.CommonJsonOptions);
                break;
            case RegistryDiscriminator.Gitlab:
                serializedCfg = JsonSerializer.Serialize(configuration, Helpers.CommonJsonOptions);
                break;
            case RegistryDiscriminator.GitHub:
                serializedCfg = JsonSerializer.Serialize(configuration, Helpers.CommonJsonOptions);
                break;

            default: throw new ArgumentException();
        }
        return serializedCfg;
    }
}

[JsonPolymorphic]
[JsonDerivedType(typeof(AWSRegistry), nameof(RegistryDiscriminator.AWS))]
[JsonDerivedType(typeof(AzureRegistry), nameof(RegistryDiscriminator.Azure))]
[JsonDerivedType(typeof(GitlabRegistry), nameof(RegistryDiscriminator.Gitlab))]
[JsonDerivedType(typeof(DockerHubRegistry), nameof(RegistryDiscriminator.DockerHub))]
[JsonDerivedType(typeof(GitHubRegistry), nameof(RegistryDiscriminator.GitHub))]
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

public class GitHubRegistry : IRegistryConfiguration
{
    [JsonInclude]
    public string Name { get; private set; }
    [JsonInclude]
    public GhcrAccountType Type { get; private set; }
    [JsonInclude]
    public string PAT { get; private set; }

    public static GitHubRegistry Create(string name, string PAT, GhcrAccountType type) =>
        new()
        {
            PAT = PAT,
            Name = name,
            Type = type
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

public enum GhcrAccountType
{
    Organization,
    User
}