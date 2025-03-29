using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Xml.Linq;
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

    public static Registry DefaultRegistry()
    {
        return new Registry
        {
            Id = Guid.Empty,
            Name = "Docker Hub",
            Url = "https://hub.docker.com/",
            Created = DateTime.MinValue,
            Discriminator = RegistryDiscriminator.DockerHub,
            Configuration = new DockerHubRegistry()
        };
    }
}

[JsonPolymorphic]
[JsonDerivedType(typeof(AWSRegistry), nameof(RegistryDiscriminator.AWS))]
[JsonDerivedType(typeof(AzureRegistry), nameof(RegistryDiscriminator.Azure))]
[JsonDerivedType(typeof(GitlabRegistry), nameof(RegistryDiscriminator.Gitlab))]
[JsonDerivedType(typeof(DockerHubRegistry), nameof(RegistryDiscriminator.DockerHub))]
[JsonDerivedType(typeof(GitHubRegistry), nameof(RegistryDiscriminator.GitHub))]
public interface IRegistryConfiguration
{
    string GetRegistryAuth() => throw new NotImplementedException();
}

public class DockerHubRegistry : IRegistryConfiguration
{
    public readonly string RegistryUrl = "https://docker.io";
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

    public async Task<(IEnumerable<DockerHubRepository> repositories, string errorMessage)> GetRepositories(IDockerHubApi dockerHub, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = UserName, Secret = PAT }, cancellationToken);
            var repositories = await dockerHub.GetRepositories(UserName, authResponse.Access_token, 1, 100, cancellationToken: cancellationToken);
            return (repositories.Results, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(ICollection<Tag> tags, string errorMessage)> GetRepositoryTags(IDockerHubApi dockerHub, string repositoryName, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = UserName, Secret = PAT }, cancellationToken);
            var repositories = await dockerHub.TagsGET(UserName, repositoryName, authResponse.Access_token, 1, 100, cancellationToken: cancellationToken);
            return (repositories.Results, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public string GetRegistryAuth() => new RegistryAuth(UserName, PAT, RegistryUrl).GetAuth();
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
    public readonly string RegistryUrl = "https://ghcr.io";
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

    public async Task<(IEnumerable<GhcrPackageVersion> versions, string errorMessage)> GetPackageVersions(IGithubCrApi githubCrApi, string packageName, CancellationToken cancellationToken)
    {
        try
        {
            var versions = Type == GhcrAccountType.User
                        ? await githubCrApi.ListPackageVersionsForUser(packageName, PAT, Name, cancellationToken: cancellationToken)
                        : await githubCrApi.ListPackageVersionsForOrg(Name, packageName, PAT, Name, cancellationToken: cancellationToken);
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

    public string GetRegistryAuth() => new RegistryAuth(Name, PAT, RegistryUrl).GetAuth();
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



internal sealed record RegistryAuth(string Username, string Password, string Serveraddress)
{
    internal string GetAuth()
        => Convert.ToBase64String(Encoding.UTF8.GetBytes(JsonSerializer.Serialize(new RegistryAuth(Username, Password, Serveraddress))));
}
