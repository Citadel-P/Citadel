using System.Security.Claims;
using Domain.Contracts.Resources.Identity;

namespace Domain.Entities.Identity;

public sealed class User(
    string name,
    string email,
    string passwordHash,
    Guid actorId,
    Guid createdByActorId,
    DateTime? createdAt = null
    ) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string Email { get; private set; } = email;
    public string Password { get; private set; } = passwordHash;
    public Guid ActorId { get; private set; } = actorId;

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = createdAt ?? DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    #endregion

    public ICollection<Team> Teams { get; } = [];
    public ICollection<RefreshToken> RefreshTokens { get; } = [];

    public void UpdateMetadata(string? name = null, string? email = null)
    {
        if (name is not null)
            Name = name;

        if (email is not null)
            Email = email;
    }

    public void SetPasswordHash(string passwordHash) => Password = passwordHash;

    public static User FromPersistence(
        Guid id,
        string name,
        string email,
        string password,
        Guid actorId,
        Guid createdByActorId,
        DateTime createdAt)
    {
        return new User(name, email, password, actorId, createdByActorId, createdAt)
        {
            Id = id,
            Password = password
        };
    }

    public static IEnumerable<Claim> GetJwtClaims(UserAuthInfo userAuthInfo) 
    {
        yield return new Claim("name", userAuthInfo.Name);
        yield return new Claim("email", userAuthInfo.Email);
        yield return new Claim("actorId", userAuthInfo.ActorId.ToString());
        yield return new Claim("sub", userAuthInfo.Id.ToString());
        yield return new Claim("jti", Guid.CreateVersion7().ToString());

        foreach (var role in userAuthInfo.Roles)
        {
            yield return new Claim("role", role);
        }
    }

}
