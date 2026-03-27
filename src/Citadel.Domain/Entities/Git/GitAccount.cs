using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Git;

public class GitAccount : IAuditedEntity
{
    public GitAccount(
        string name,
        string domain,
        GitTransport transport,
        GitAuthType authType,
        Guid createdByActorId,
        GitAuthConfiguration configuration)
    {
        Validate(transport, authType, configuration);

        Name = name;
        Domain = domain;
        Transport = transport;
        AuthType = authType;
        CreatedByActorId = createdByActorId;
        Configuration = configuration;
    }

    private GitAccount()
    {
    }

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = string.Empty;
    public string Domain { get; private set; } = string.Empty;
    public GitTransport Transport { get; private set; }
    public GitAuthType AuthType { get; private set; }

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; }
    #endregion

    public GitAuthConfiguration Configuration { get; private set; } = new TokenAuth(string.Empty);

    public void PartialUpdate(
        string? name = null,
        string? domain = null,
        GitTransport? transport = null,
        GitAuthType? authType = null,
        GitAuthConfiguration? configuration = null)
    {
        var updatedName = name ?? Name;
        var updatedDomain = domain ?? Domain;
        var updatedTransport = transport ?? Transport;
        var updatedAuthType = authType ?? AuthType;
        var updatedConfiguration = configuration ?? Configuration;

        Validate(updatedTransport, updatedAuthType, updatedConfiguration);

        if (name is not null)
            Name = updatedName;

        if (domain is not null)
            Domain = updatedDomain;

        if (transport is not null)
            Transport = updatedTransport;

        if (authType is not null)
            AuthType = updatedAuthType;

        if (configuration is not null)
            Configuration = updatedConfiguration;
    }

    public static GitAccount FromPersistence(
        Guid id,
        string name,
        string domain,
        GitTransport transport,
        GitAuthType authType,
        DateTime createdAt,
        Guid createdByActorId,
        GitAuthConfiguration configuration)
    {
        return new GitAccount(name, domain, transport, authType, createdByActorId, configuration)
        {
            Id = id,
            CreatedAt = createdAt
        };
    }

    private static void Validate(GitTransport transport, GitAuthType authType, GitAuthConfiguration auth)
    {
        ValidateAuthType(authType, auth);

        if (transport == GitTransport.Ssh && auth is not SshKeyAuth)
            throw new InvalidOperationException("SSH requires SSH key authentication");

        if (transport != GitTransport.Ssh && auth is SshKeyAuth)
            throw new InvalidOperationException("SSH auth cannot be used with HTTP/HTTPS");
    }

    private static void ValidateAuthType(GitAuthType authType, GitAuthConfiguration auth)
    {
        if ((authType, auth) is not (
                (GitAuthType.Basic, BasicAuth)
                or (GitAuthType.Token, TokenAuth)
                or (GitAuthType.Token, TokenAuth)
                or (GitAuthType.SshKey, SshKeyAuth)))
        {
            throw new InvalidOperationException($"Configuration type '{auth.GetType().Name}' does not match auth type '{authType}'.");
        }
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(BasicAuth), nameof(GitAuthType.Basic))]
[JsonDerivedType(typeof(TokenAuth), nameof(GitAuthType.Token))]
[JsonDerivedType(typeof(SshKeyAuth), nameof(GitAuthType.SshKey))]
public abstract record GitAuthConfiguration;

public record TokenAuth(string Token) : GitAuthConfiguration;
public record BasicAuth(string Username, string Password) : GitAuthConfiguration;
public record SshKeyAuth(string Username, string PrivateKey, string? Passphrase) : GitAuthConfiguration;
