using Application.Configs;
using Application.Services;
using Microsoft.Extensions.Options;

namespace Tests.Unit;

internal static class TestAdoptionFingerprint
{
    public static IAdoptionFingerprintService Create(byte keyByte = 0x2A)
        => new AdoptionFingerprintService(Options.Create(new SecretsConfiguration
        {
            EncryptionKey = Convert.ToBase64String(Enumerable.Repeat(keyByte, SecretsConfiguration.EncryptionKeySize).ToArray())
        }));
}
