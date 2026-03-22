using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Git;

[method: JsonConstructor]
public class GitAccount(
    string name,
    string domain,
    GitAuthType authType,
    Guid createdByActorId,
    GitAccountConfiguration configuration) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string Domain { get; private set; } = domain;
    public GitAuthType AuthType { get; private set; } = authType;

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    #endregion

    public GitAccountConfiguration Configuration { get; private set; } = configuration;

    public void PartialUpdate(
        string? name = null,
        string? domain = null,
        GitAuthType? authType = null,
        GitAccountConfiguration? configuration = null)
    {
        var updatedName = name ?? Name;
        var updatedDomain = domain ?? Domain;
        var updatedAuthType = authType ?? AuthType;
        var updatedConfiguration = configuration ?? Configuration;

        if (name is not null)
            Name = updatedName;

        if (domain is not null)
            Domain = updatedDomain;

        if (authType is not null)
            AuthType = updatedAuthType;

        if (configuration is not null)
            Configuration = updatedConfiguration;
    }

    public static GitAccount FromPersistence(
        Guid id,
        string name,
        string domain,
        GitAuthType authType,
        DateTime createdAt,
        Guid createdByActorId,
        GitAccountConfiguration configuration)
    {
        return new GitAccount(name, domain, authType, createdByActorId, configuration)
        {
            Id = id,
            CreatedAt = createdAt
        };
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(NoAuthAccount), nameof(GitAuthType.None))]
[JsonDerivedType(typeof(GitSshAccount), nameof(GitAuthType.Ssh))]
[JsonDerivedType(typeof(GitHttpAccount), nameof(GitAuthType.Https))]
public abstract record GitAccountConfiguration;

public record NoAuthAccount : GitAccountConfiguration;
public record GitSshAccount(string Username, string PrivateKey, string? Passphrase = null) : GitAccountConfiguration;
public record GitHttpAccount(bool? AuthEnabled = false, string? Username = null, string? Token = null) : GitAccountConfiguration;
