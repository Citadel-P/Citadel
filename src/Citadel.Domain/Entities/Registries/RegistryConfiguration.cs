using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(AWSRegistry), nameof(RegistryType.AWS))]
[JsonDerivedType(typeof(AzureRegistry), nameof(RegistryType.Azure))]
[JsonDerivedType(typeof(GitlabRegistry), nameof(RegistryType.Gitlab))]
[JsonDerivedType(typeof(DockerHubRegistry), nameof(RegistryType.DockerHub))]
[JsonDerivedType(typeof(GitHubRegistry), nameof(RegistryType.GitHub))]
[JsonDerivedType(typeof(CustomRegistry), nameof(RegistryType.Custom))]
public abstract record RegistryConfiguration
{
    public abstract string? GetRegistryAuth(string registryHost);
}

[method: JsonConstructor]
public record GitlabRegistry(string UserName, string PAT, string InstanceUrl) : RegistryConfiguration
{
    public static GitlabRegistry Create(string userName, string PAT, string instanceUrl)
        => new(userName, PAT, instanceUrl);
    public override string GetRegistryAuth(string registryHost) => throw new NotImplementedException();
}

[method: JsonConstructor]
public record GitHubRegistry(string NameSpace, bool? GhcrAuthEnabled = false, string? PAT = null) : RegistryConfiguration
{
    public override string? GetRegistryAuth(string registryHost) => GhcrAuthEnabled == true
        ? new RegistryAuth(NameSpace, PAT ?? string.Empty, registryHost).GetAuth()
        : null;

    public static GitHubRegistry Create(string nameSpace, bool? ghcrAuthEnabled, string? PAT) => new(nameSpace, ghcrAuthEnabled, PAT);
}

[method: JsonConstructor]
public record DockerHubRegistry(string? UserName = null, string? PAT = null) : RegistryConfiguration
{
    public override string? GetRegistryAuth(string registryHost) => new RegistryAuth(UserName ?? string.Empty, PAT ?? string.Empty, registryHost).GetAuth();

    public static DockerHubRegistry Create(string userName, string PAT) => new(userName, PAT);
}

[method: JsonConstructor]
public record CustomRegistry(bool? AuthEnabled = false, string? UserName = null, string? Password = null) : RegistryConfiguration
{
    public override string? GetRegistryAuth(string registryHost) => AuthEnabled == true
        ? new RegistryAuth(UserName ?? string.Empty, Password ?? string.Empty, registryHost).GetAuth()
        : null;

    public static CustomRegistry Create(bool? authEnabled, string? userName, string? PAT) => new(authEnabled, userName, PAT);
}


[method: JsonConstructor]
public record AzureRegistry(string UserName, string Password) : RegistryConfiguration
{
    public static AzureRegistry Create(string userName, string password) =>
        new(userName, password);
    public override string GetRegistryAuth(string registryHost) => throw new NotImplementedException();
}

[method: JsonConstructor]
public record AWSRegistry(string AccessKey, bool AuthenticationRequired, string SecretAccessKey, string Region)
    : RegistryConfiguration
{
    public override string GetRegistryAuth(string registryHost) => throw new NotImplementedException();
    public static AWSRegistry Create(bool authenticationRequired, string accessKey, string secretAccessKey, string region)
        => new(accessKey, authenticationRequired, secretAccessKey, region);
}
