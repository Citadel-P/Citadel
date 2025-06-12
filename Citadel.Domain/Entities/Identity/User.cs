using System.Security.Claims;
using System.Security.Cryptography;

namespace Domain.Entities.Identity;

public class User
{
    public Guid Id { get; private set; }
    public string Name { get; private set; } = null!;
    public string Email { get; private set; } = null!;
    public string Password { get; private set; } = null!;
    public DateTime CreatedAt { get; private set; }
    public DateTime UpdatedAt { get; private set; }
    public ICollection<Team> Teams { get; } = [];
    public ICollection<RefreshToken> RefreshTokens { get; } = [];

    /// <summary>
    /// Factory method to create a user
    /// </summary>
    /// <param name="name">user name</param>
    /// <param name="email">email</param>
    /// <param name="password">plain text password</param>
    public static User Create(string name, string email, string password, Guid? id = null, DateTime? createdAt = null) => new()
    {
        Id = id ?? Guid.CreateVersion7(),
        Name = name,
        Email = email,
        CreatedAt = createdAt ?? DateTime.UtcNow,
        Password = HashPassword(password),
    };

    /// <summary>
    /// Check user password is valid
    /// </summary>
    public bool IsValidPassword(string plainTextPassword)
    {
        // Extract the bytes
        byte[] hashBytes = Convert.FromBase64String(Password);

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

    public IEnumerable<Claim> GetJwtClaims() 
    {
        yield return new Claim("name", Name);
        yield return new Claim("email", Email);
        yield return new Claim("sub", Id.ToString());
        yield return new Claim("jti", Guid.CreateVersion7().ToString());

        yield return new Claim("permissions", string.Join(";", GetPermissions()));
        foreach (var role in Teams.Select(s => s.Role.Name))
        {
            yield return new Claim("role", role);
        }
    }

    /// <summary>
    /// Get user permissions
    /// </summary>
    private IEnumerable<AppPermission> GetPermissions()
    {
        var permissions = new HashSet<AppPermission>();
        foreach (var team in Teams)
        {
            foreach (var permission in team.Role.Permissions)
            {
                permissions.Add(permission.PermissionCode);
            }
        }

        return permissions;
    }

    /// <summary>
    /// Hash a password
    /// </summary>
    /// <param name="plainTextPassword">The plain text password</param>
    /// <returns> A secure base64 hashed password</returns>
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
        using Rfc2898DeriveBytes pbkdf2 = new(plainTextPassword, salt, 10000, HashAlgorithmName.SHA256);
        return pbkdf2.GetBytes(20);
    }
}