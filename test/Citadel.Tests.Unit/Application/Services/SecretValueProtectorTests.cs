using Application.Configs;
using Application.Services;
using Hosting.Common;
using Microsoft.Extensions.Options;
using System.Security.Cryptography;

namespace Tests.Unit.Application.Services;

public sealed class SecretValueProtectorTests
{
    [Fact]
    public void Protect_Should_RoundTrip_With_Dedicated_Encryption_Key()
    {
        var protector = CreateProtector(1);

        var protectedValue = protector.Protect("super-secret");
        var plaintext = protector.Unprotect(protectedValue);

        Assert.Equal("super-secret", plaintext);
        Assert.NotEqual("super-secret", protectedValue);
    }

    [Fact]
    public void Unprotect_Should_Fail_When_Encryption_Key_Differs()
    {
        var firstProtector = CreateProtector(1);
        var secondProtector = CreateProtector(2);
        var protectedValue = firstProtector.Protect("super-secret");

        Assert.ThrowsAny<CryptographicException>(() => secondProtector.Unprotect(protectedValue));
    }

    [Theory]
    [InlineData("not-base64")]
    public void SecretsConfiguration_Should_Reject_Invalid_Configured_Encryption_Key(string value)
    {
        Assert.False(SecretsConfiguration.IsValidEncryptionKey(value));
    }

    [Fact]
    public void SecretsConfiguration_Should_Allow_Empty_Encryption_Key_For_File_Fallback()
    {
        Assert.True(SecretsConfiguration.IsValidEncryptionKey(""));
    }

    [Fact]
    public void SecretsConfiguration_Should_Reject_Key_With_Wrong_Length()
    {
        var key = Convert.ToBase64String(RandomNumberGenerator.GetBytes(16));

        Assert.False(SecretsConfiguration.IsValidEncryptionKey(key));
    }

    [Fact]
    public void GetSecretEncryptionKeyFromFile_Should_Generate_And_Reuse_Base64_32_Byte_Key()
    {
        var path = Path.Combine(Path.GetTempPath(), $"citadel-secret-key-{Guid.NewGuid():N}");
        try
        {
            var first = Helpers.GetSecretEncryptionKeyFromFile(path);
            var second = Helpers.GetSecretEncryptionKeyFromFile(path);

            Assert.Equal(first, second);
            Assert.True(SecretsConfiguration.TryGetEncryptionKey(first, out var key));
            Assert.Equal(SecretsConfiguration.EncryptionKeySize, key.Length);
        }
        finally
        {
            if (File.Exists(path))
                File.Delete(path);
        }
    }

    [Fact]
    public async Task GetJwtSecretFromFile_Should_Generate_One_Key_For_Concurrent_Callers()
    {
        var root = Path.Combine(Path.GetTempPath(), $"citadel-jwt-key-{Guid.NewGuid():N}");
        var path = Path.Combine(root, "data", "jwtsecret");
        try
        {
            var calls = Enumerable.Range(0, 32)
                .Select(_ => Task.Run(() => Helpers.GetJwtSecretFromFile(path)))
                .ToArray();

            var keys = await Task.WhenAll(calls);

            Assert.Single(keys.Distinct(StringComparer.Ordinal));
            Assert.Equal(keys[0], await File.ReadAllTextAsync(path, TestContext.Current.CancellationToken));
        }
        finally
        {
            if (Directory.Exists(root))
                Directory.Delete(root, recursive: true);
        }
    }

    private static SecretValueProtector CreateProtector(byte fill)
    {
        var key = Enumerable.Repeat(fill, SecretsConfiguration.EncryptionKeySize).ToArray();
        var config = new SecretsConfiguration
        {
            EncryptionKey = Convert.ToBase64String(key)
        };

        return new SecretValueProtector(Options.Create(config));
    }
}
