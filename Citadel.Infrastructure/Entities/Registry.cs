using System.Diagnostics.CodeAnalysis;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Infrastructure.DockerHub;
using Infrastructure.GithubCr;
using Refit;

namespace Infrastructure.Entities;

/// <summary>
/// Represent a container registry
/// </summary>
[method: JsonConstructor]
public class Registry(string name, string url, RegistryType type, RegistryConfigurationBase configuration)
{
    public static readonly string DefaultRegistryName = "Docker Hub";
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string Url { get; private set; } = url;
    public DateTime Created { get; private set; } = DateTime.UtcNow;

    /// <summary>
    /// The name of the type that will be serialized/deserialized
    /// </summary>
    public RegistryType Type { get; private set; } = type;

    /// <summary>
    /// The registry configuration
    /// </summary>
    public RegistryConfigurationBase Configuration { get; private set; } = configuration;

    public void PartialUpdate(string? name = null, string? url = null, RegistryConfigurationBase? configuration = null)
    {
        if (name != null) Name = name;
        if (url != null) Url = url;
        if (configuration != null) Configuration = configuration;
    }

    public static Registry DefaultRegistry()
    {
        var registry = new Registry(
            name: DefaultRegistryName,
            url: "https://hub.docker.com/",
            type: RegistryType.DockerHub,
            configuration: new DockerHubRegistry())
        {
            Id = Guid.Empty,
            Created = DateTime.MinValue // Set to a default value for the default registry
        };

        return registry;
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(AWSRegistry), nameof(RegistryType.AWS))]
[JsonDerivedType(typeof(AzureRegistry), nameof(RegistryType.Azure))]
[JsonDerivedType(typeof(GitlabRegistry), nameof(RegistryType.Gitlab))]
[JsonDerivedType(typeof(DockerHubRegistry), nameof(RegistryType.DockerHub))]
[JsonDerivedType(typeof(GitHubRegistry), nameof(RegistryType.GitHub))]
public abstract class RegistryConfigurationBase
{
    public virtual string RegistryUrl { get; private set;  } = null!;
    public abstract string GetRegistryAuth();
}

[method: JsonConstructor]
public class DockerHubRegistry(string? userName = null, string? pat = null) : RegistryConfigurationBase
{
    public override string RegistryUrl => "https://docker.io";
    public string UserName { get; } = userName ?? string.Empty;
    public string PAT { get; } = pat ?? string.Empty;
    public static DockerHubRegistry Create(string userName, string PAT) => new (userName, PAT);

    public async Task<(bool success, string? errorMessage)> CanConnect(IDockerHubApi dockerHub, CancellationToken cancellationToken)
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

    public async Task<(IEnumerable<DockerHubRepository>? repositories, string? errorMessage)> GetRepositories(IDockerHubApi dockerHub, CancellationToken cancellationToken)
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

    public async Task<(ICollection<Tag>? tags, string? errorMessage)> GetRepositoryTags(IDockerHubApi dockerHub, string repositoryName, CancellationToken cancellationToken)
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

    public override string GetRegistryAuth() => new RegistryAuth(UserName, PAT, RegistryUrl).GetAuth();
}

[method: JsonConstructor]
public class AzureRegistry(string userName, string password) : RegistryConfigurationBase
{
    public override string RegistryUrl => string.Empty;
    public string UserName { get; } = userName;
    public string Password { get; } = password;

    public static AzureRegistry Create(string userName, string password) => 
        new (userName, password);

    public override string GetRegistryAuth()
    {
        throw new NotImplementedException();
    }
}

[method: JsonConstructor]
public class GitHubRegistry(string name, GhcrAccountType? type, string pat) : RegistryConfigurationBase
{
    public override string RegistryUrl => "https://ghcr.io";
    public string Name { get; } = name;
    public GhcrAccountType? Type { get; } = type;
    public string PAT { get; } = pat;

    public static GitHubRegistry Create(string name, string PAT, GhcrAccountType type) =>
        new (name, type, PAT);

    public async Task<(bool success, string? errorMessage)> CanConnect(IGithubCrApi githubCrApi, CancellationToken cancellationToken)
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

    public async Task<(IEnumerable<GhcrPackage>? packages, string? errorMessage)> GetPackages(IGithubCrApi githubCrApi, CancellationToken cancellationToken)
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

    public async Task<(IEnumerable<GhcrPackageVersion>? versions, string? errorMessage)> GetPackageVersions(IGithubCrApi githubCrApi, string packageName, CancellationToken cancellationToken)
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

    public override string GetRegistryAuth() => new RegistryAuth(Name, PAT, RegistryUrl).GetAuth();
}

[method: JsonConstructor]
public class AWSRegistry(string accessKey, bool authenticationRequired, string secretAccessKey, string region) : RegistryConfigurationBase
{
    public override string RegistryUrl => string.Empty;
    /// <summary>
    /// If true, the credential bellow should be specified in order to connect to a private AWS registry
    /// </summary>
    public bool AuthenticationRequired { get; } = authenticationRequired;
    public string AccessKey { get; } = accessKey;
    public string SecretAccessKey { get; } = secretAccessKey;
    public string Region { get; } = region;

    public static AWSRegistry Create(bool authenticationRequired, string accessKey, string secretAccessKey, string region)
        => new (accessKey, authenticationRequired, secretAccessKey, region);

    public override string GetRegistryAuth()
    {
        throw new NotImplementedException();
    }
}

[method: JsonConstructor]
public class GitlabRegistry(string userName, string pat, string instanceUrl) : RegistryConfigurationBase
{
    public override string RegistryUrl => string.Empty;
    public string UserName { get; } = userName;
    public string PAT { get; } = pat;
    public string InstanceUrl { get; } = instanceUrl;

    public static GitlabRegistry Create(string userName, string PAT, string instanceUrl) 
        => new (userName, PAT, instanceUrl);

    public override string GetRegistryAuth()
    {
        throw new NotImplementedException();
    }
}

internal sealed record RegistryAuth(string Username, string Password, string Serveraddress)
{
    internal string GetAuth()
        => Convert.ToBase64String(Encoding.UTF8.GetBytes(JsonSerializer.Serialize(new RegistryAuth(Username, Password, Serveraddress), typeof(RegistryAuth), RegistryAuthContext.Default)));
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(RegistryAuth))]
internal partial class RegistryAuthContext : JsonSerializerContext
{
}

