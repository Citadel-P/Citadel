using System.Text.Json.Serialization;

namespace Infrastructure.Entities;

/// <summary>
/// Represent a container registry
/// </summary>
public class Registry
{
    public Guid Id { get; set; }
    public string Name { get; set; }
    public string Url { get; set; }
    public DateTime Created { get; set; }

    /// <summary>
    /// The name of the type that will be serialized/deserialized
    /// </summary>
    public RegistryDiscriminator Discriminator { get; set; }

    /// <summary>
    /// The registry configuration serialized as a json string.
    /// Contains an implementation of the <see cref="IRegistryConfiguration"/>
    /// </summary>
    public string Configuration { get; set; }
}

[JsonDerivedType(typeof(AWSRegistry), (int)RegistryDiscriminator.AWS)]
[JsonDerivedType(typeof(AzureRegistry), (int)RegistryDiscriminator.Azure)]
[JsonDerivedType(typeof(GitlabRegistry), (int)RegistryDiscriminator.Gitlab)]
[JsonDerivedType(typeof(CustomRegistry), (int)RegistryDiscriminator.Custom)]
[JsonDerivedType(typeof(DockerHubRegistry), (int)RegistryDiscriminator.DockerHub)]
public interface IRegistryConfiguration
{ }

public class DockerHubRegistry : IRegistryConfiguration
{
    public string UserName { get; set; }
    public string PAT { get; set; }
}

public class AzureRegistry : IRegistryConfiguration
{
    public string UserName { get; set; }
    public string Password { get; set; }
}

public class AWSRegistry : IRegistryConfiguration
{
    /// <summary>
    /// If true, the credential bellow should be specified in order to connect to a private AWS registry
    /// </summary>
    public bool AuthenticationRequired { get; set; }

    public string AccessKey { get; set; }
    public string SecretAccessKey { get; set; }
    public string Region { get; set; }
}

public class GitlabRegistry : IRegistryConfiguration
{
    public string UserName { get; set; }
    public string PAT { get; set; }
    public string InstanceUrl { get; set; }
}

public class CustomRegistry : IRegistryConfiguration
{
    /// <summary>
    /// If true, the credential bellow should be specified in order to connect to a private AWS registry
    /// </summary>
    public bool AuthenticationRequired { get; set; }

    public string UserName { get; set; }
    public string Password { get; set; }
}