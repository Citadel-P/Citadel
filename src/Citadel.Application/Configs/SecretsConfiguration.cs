using System.Security.Cryptography;
using Hosting.Common;

namespace Application.Configs;

public sealed class SecretsConfiguration
{
    public const string SectionName = "Secrets";
    public const int EncryptionKeySize = 32;

    public string EncryptionKey { get; set; } = string.Empty;

    public byte[] GetEncryptionKey()
    {
        var configuredKey = string.IsNullOrWhiteSpace(EncryptionKey)
            ? Helpers.GetSecretEncryptionKeyFromFile()
            : EncryptionKey;

        if (!TryGetEncryptionKey(configuredKey, out var key))
            throw new CryptographicException("Secrets:EncryptionKey or generated secret encryption key file must be a base64-encoded 32-byte key.");

        return key;
    }

    public static bool IsValidEncryptionKey(string? value)
        => string.IsNullOrWhiteSpace(value) || TryGetEncryptionKey(value, out _);

    public static bool TryGetEncryptionKey(string? value, out byte[] key)
    {
        key = [];

        if (string.IsNullOrWhiteSpace(value))
            return false;

        try
        {
            var decoded = Convert.FromBase64String(value.Trim());
            if (decoded.Length != EncryptionKeySize)
                return false;

            key = decoded;
            return true;
        }
        catch (FormatException)
        {
            return false;
        }
    }
}
