using System.Security.Claims;
using System.Security.Cryptography;
using Domain.Contracts.Resources.Identity;

namespace Domain.Entities.Identity;

public sealed class User(
    string name,
    string email,
    string password,
    Guid actorId,
    Guid createdByActorId,
    DateTime? createdAt = null
    ) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string Email { get; private set; } = email;
    public string Password { get; private set; } = HashPassword(password);
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

    public void SetPassword(string password) => Password = HashPassword(password);

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

    /// <summary>
    /// Check user password is valid
    /// </summary>
    public static bool IsValidPassword(string plainTextPassword, string originalPassword)
    {
        // Extract the bytes
        byte[] hashBytes = Convert.FromBase64String(originalPassword);

        // Get the salt
        byte[] salt = new byte[16];
        Array.Copy(hashBytes, 0, salt, 0, 16);
        byte[] hash = ComputeHash(plainTextPassword, salt);

        // Compare the results
        for (int i = 0; i < 20; i++)
        {
            if (hashBytes[i + 16] != hash[i])
            {
                return false;
            }
        }

        return true;
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

        foreach (var permission in userAuthInfo.Permissions)
        {
            yield return new Claim("permission", permission.ToString()!);
        }
    }

    /// <summary>
    /// Hash a password
    /// </summary>
    /// <param name="plainTextPassword">The plain text password</param>
    /// <returns>A securely base64-encoded hashed password</returns>
    private static string HashPassword(string plainTextPassword)
    {
        byte[] salt = RandomNumberGenerator.GetBytes(16);
        byte[] hash = ComputeHash(plainTextPassword, salt);

        // Combine the salt and password bytes
        byte[] hashBytes = new byte[36];
        Array.Copy(salt, 0, hashBytes, 0, 16);
        Array.Copy(hash, 0, hashBytes, 16, hash.Length);

        // return a stringify hash for storage
        return Convert.ToBase64String(hashBytes);
    }

    private static byte[] ComputeHash(string plainTextPassword, byte[] salt)
    {
        return Rfc2898DeriveBytes.Pbkdf2(plainTextPassword, salt, 10000, HashAlgorithmName.SHA256, 20);
    }
}