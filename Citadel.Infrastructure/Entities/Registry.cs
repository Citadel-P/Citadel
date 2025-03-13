using System.Text.Json.Serialization;
using Infrastructure.DockerHub;
using Infrastructure.GithubCr;
using Refit;

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
    /// The registry configuration
    /// </summary>
    public IRegistryConfiguration Configuration { get; set; }

    public static Registry Create(string name, string url, RegistryDiscriminator discriminator, IRegistryConfiguration configuration) 
        => new ()
        {
            Id = Guid.CreateVersion7(),
            Created = DateTime.UtcNow,
            Name = name,
            Url = url,
            Discriminator = discriminator,
            Configuration = configuration 
        };

    public void PartialUpdate(string name = null, string url = null, IRegistryConfiguration configuration = null)
    {
        if (name != null) Name = name;
        if (url != null) Url = url;
        if (configuration != null) Configuration = configuration;
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

    public async Task<(bool success, string errorMessage)> CanConnect(IDockerHubApi dockerHub, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = UserName, Secret = PAT }, cancellationToken);
            return (true, null);
        }
        catch (ApiException ex)
        {
            return (false, 
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    
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
    public GhcrAccountType? Type { get; private set; }
    [JsonInclude]
    public string PAT { get; private set; }

    public static GitHubRegistry Create(string name, string PAT, GhcrAccountType type) =>
        new()
        {
            PAT = PAT,
            Name = name,
            Type = type
        };

    public async Task<(bool success, string errorMessage)> CanConnect(IGithubCrApi githubCrApi, CancellationToken cancellationToken)
    {
        try
        {
            var packages = Type == GhcrAccountType.Organization
                    ? await githubCrApi.ListOrgPackages(Name, PAT, Name, cancellationToken)
                    : await githubCrApi.ListUserPackages(Name, PAT, Name, cancellationToken);
            return (true, null);
        }
        catch (ApiException ex)
        {
            return (false,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid GitHub credentials, please verify your input."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<GhcrPackage> packages, string errorMessage)> GetPackages(IGithubCrApi githubCrApi, CancellationToken cancellationToken)
    {
        try
        {
            var packages = Type == GhcrAccountType.User
                ? await githubCrApi.ListUserPackages(Name, PAT, Name, cancellationToken)
                : await githubCrApi.ListOrgPackages(Name, PAT, Name, cancellationToken);
            return (packages, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<GhcrPackageVersion> packages, string errorMessage)> GetPackageVersions(IGithubCrApi githubCrApi, CancellationToken cancellationToken)
    {
        try
        {
            var versions = Type == GhcrAccountType.User
                        ? await githubCrApi.ListPackageVersionsForUser(Name, PAT, Name, cancellationToken)
                        : await githubCrApi.ListPackageVersionsForOrg(Name, Name, PAT, Name, cancellationToken);
            return (versions, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }
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