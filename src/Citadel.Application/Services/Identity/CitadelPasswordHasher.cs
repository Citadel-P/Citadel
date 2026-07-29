using System.Security.Cryptography;
using Microsoft.AspNetCore.Identity;
using Microsoft.Extensions.Options;

namespace Application.Services.Identity;

public interface ICitadelPasswordHasher
{
    string Hash(string password);

    bool Verify(string password, string encodedHash);
}

internal sealed class CitadelPasswordHasher : ICitadelPasswordHasher
{
    internal const int IterationCount = 220_000;

    private static readonly PasswordHasherMarker Marker = new();

    private readonly PasswordHasher<PasswordHasherMarker> hasher = new(
        Options.Create(new PasswordHasherOptions
        {
            CompatibilityMode = PasswordHasherCompatibilityMode.IdentityV3,
            IterationCount = IterationCount
        }));

    public string Hash(string password) => hasher.HashPassword(Marker, password);

    public bool Verify(string password, string encodedHash)
    {
        if (string.IsNullOrEmpty(encodedHash))
            return false;

        try
        {
            return hasher.VerifyHashedPassword(Marker, encodedHash, password)
                == PasswordVerificationResult.Success;
        }
        catch (FormatException)
        {
            return false;
        }
        catch (CryptographicException)
        {
            return false;
        }
        catch (IndexOutOfRangeException)
        {
            return false;
        }
    }

    private sealed class PasswordHasherMarker;
}
