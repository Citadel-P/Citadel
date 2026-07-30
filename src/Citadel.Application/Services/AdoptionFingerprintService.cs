using Application.Configs;
using Microsoft.Extensions.Options;
using System.Security.Cryptography;
using System.Text;

namespace Application.Services;

internal interface IAdoptionFingerprintService
{
    string Sign(string value);
    bool Matches(string expectedSignature, string providedSignature);
}

internal sealed class AdoptionFingerprintService : IAdoptionFingerprintService
{
    private const string Purpose = "citadel:resource-adoption-fingerprint:v1";
    private readonly byte[] signingKey;

    public AdoptionFingerprintService(IOptions<SecretsConfiguration> options)
    {
        signingKey = HMACSHA256.HashData(
            options.Value.GetEncryptionKey(),
            Encoding.UTF8.GetBytes(Purpose));
    }

    public string Sign(string value)
        => Convert.ToHexString(HMACSHA256.HashData(signingKey, Encoding.UTF8.GetBytes(value)));

    public bool Matches(string expectedSignature, string providedSignature)
    {
        if (expectedSignature.Length != 64 || providedSignature.Length != 64)
            return false;

        try
        {
            var expected = Convert.FromHexString(expectedSignature);
            var provided = Convert.FromHexString(providedSignature);
            return CryptographicOperations.FixedTimeEquals(expected, provided);
        }
        catch (FormatException)
        {
            return false;
        }
    }
}
