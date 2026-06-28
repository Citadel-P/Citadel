using Application.Configs;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;

namespace Application.Services;

internal interface IConfigurationResolver
{
    Task<Result<ResolvedConfiguration>> ResolveAsync(ConfigurationScope scope, Guid resourceId, CancellationToken cancellationToken);
}

internal interface ISecretValueProtector
{
    string Protect(string value);
    string Unprotect(string protectedValue);
}

internal interface ISecretRedactor
{
    string Redact(string? value, IEnumerable<string> secrets);
}

internal sealed partial class ConfigurationResolver(IServiceScopeFactory scopeFactory, ISecretValueProtector secretValueProtector) : IConfigurationResolver
{
    private static readonly Regex NameRegex = GetNameRegex();

    public async Task<Result<ResolvedConfiguration>> ResolveAsync(ConfigurationScope scope, Guid resourceId, CancellationToken cancellationToken)
    {
        await using var serviceScope = scopeFactory.CreateAsyncScope();
        var uow = serviceScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var entries = await uow.ConfigurationEntries.GetEffectiveEntriesAsync(scope, resourceId, cancellationToken);

        var effective = MergeEffective(entries);
        var environment = new Dictionary<string, string>(StringComparer.Ordinal);
        var resolvedEntries = new List<ResolvedConfigurationEntry>();
        var redactionValues = new List<string>();
        var variableCount = 0;
        var secretCount = 0;

        foreach (var entry in effective)
        {
            if (!NameRegex.IsMatch(entry.Name))
                return Result.Failure<ResolvedConfiguration>($"Configuration key '{entry.Name}' is not a valid environment variable name.");

            try
            {
                entry.Validate();
            }
            catch (ArgumentException ex)
            {
                return Result.Failure<ResolvedConfiguration>(ex.Message);
            }

            if (entry.Kind == ConfigurationEntryKind.Variable)
            {
                variableCount++;
                var variableValue = entry.Value ?? string.Empty;
                environment[entry.Name] = variableValue;
                resolvedEntries.Add(new ResolvedConfigurationEntry(entry.Name, entry.Kind, variableValue));
                continue;
            }

            secretCount++;
            var secret = await uow.SecretDefinitions.GetAsync(entry.SecretId!.Value, cancellationToken);
            if (secret is null)
                return Result.Failure<ResolvedConfiguration>($"Secret {entry.Name} is not available.");

            if (secret.ProviderType != SecretProviderType.InternalEncrypted)
                return Result.Failure<ResolvedConfiguration>($"Secret provider {secret.ProviderType} is not implemented.");

            var value = await uow.SecretDefinitions.GetInternalValueAsync(secret.Id, cancellationToken);
            if (value is null)
                return Result.Failure<ResolvedConfiguration>($"Secret {entry.Name} is not available.");

            var plaintext = secretValueProtector.Unprotect(value.EncryptedValue);
            environment[entry.Name] = plaintext;
            resolvedEntries.Add(new ResolvedConfigurationEntry(entry.Name, entry.Kind, plaintext, secret.Name));
            if (!string.IsNullOrEmpty(plaintext))
            {
                redactionValues.Add(plaintext);
            }
        }

        return new ResolvedConfiguration(
            EnvironmentVariables: [.. environment.Select(kv => $"{kv.Key}={kv.Value}")],
            Entries: resolvedEntries,
            RedactionValues: [.. redactionValues],
            VariableCount: variableCount,
            SecretCount: secretCount);
    }

    private static IReadOnlyList<ConfigurationEntry> MergeEffective(IEnumerable<ConfigurationEntry> entries)
    {
        var effective = new Dictionary<string, ConfigurationEntry>(StringComparer.Ordinal);
        foreach (var entry in entries.OrderBy(x => x.Scope == ConfigurationScope.Global ? 0 : 1))
        {
            effective[entry.Name] = entry;
        }

        return effective.Values.OrderBy(x => x.Name, StringComparer.Ordinal).ToArray();
    }

    [GeneratedRegex("^[A-Za-z_][A-Za-z0-9_]*$", RegexOptions.Compiled)]
    private static partial Regex GetNameRegex();
}

internal sealed class SecretValueProtector(IOptions<JwtConfiguration> jwtOptions) : ISecretValueProtector
{
    private const int NonceSize = 12;
    private const int TagSize = 16;

    public string Protect(string value)
    {
        var plaintext = Encoding.UTF8.GetBytes(value);
        var nonce = RandomNumberGenerator.GetBytes(NonceSize);
        var ciphertext = new byte[plaintext.Length];
        var tag = new byte[TagSize];

        using var aes = new AesGcm(GetKey(), TagSize);
        aes.Encrypt(nonce, plaintext, ciphertext, tag);

        var payload = new byte[NonceSize + TagSize + ciphertext.Length];
        Buffer.BlockCopy(nonce, 0, payload, 0, NonceSize);
        Buffer.BlockCopy(tag, 0, payload, NonceSize, TagSize);
        Buffer.BlockCopy(ciphertext, 0, payload, NonceSize + TagSize, ciphertext.Length);

        return Convert.ToBase64String(payload);
    }

    public string Unprotect(string protectedValue)
    {
        var payload = Convert.FromBase64String(protectedValue);
        if (payload.Length < NonceSize + TagSize)
            throw new CryptographicException("Invalid secret payload.");

        var nonce = payload[..NonceSize];
        var tag = payload[NonceSize..(NonceSize + TagSize)];
        var ciphertext = payload[(NonceSize + TagSize)..];
        var plaintext = new byte[ciphertext.Length];

        using var aes = new AesGcm(GetKey(), TagSize);
        aes.Decrypt(nonce, ciphertext, tag, plaintext);

        return Encoding.UTF8.GetString(plaintext);
    }

    private byte[] GetKey()
    {
        var keyMaterial = string.IsNullOrWhiteSpace(jwtOptions.Value.Key)
            ? Helpers.GetJwtSecretFromFile()
            : jwtOptions.Value.Key;

        if (string.IsNullOrWhiteSpace(keyMaterial))
            throw new InvalidOperationException("Secret encryption key material is missing.");

        return SHA256.HashData(Encoding.UTF8.GetBytes(keyMaterial));
    }
}

internal sealed class SecretRedactor : ISecretRedactor
{
    public string Redact(string? value, IEnumerable<string> secrets)
    {
        if (string.IsNullOrEmpty(value))
            return string.Empty;

        var redacted = value;
        foreach (var secret in secrets.Where(x => !string.IsNullOrEmpty(x)).Distinct(StringComparer.Ordinal))
        {
            redacted = redacted.Replace(secret, "********", StringComparison.Ordinal);
        }

        return redacted;
    }
}

internal sealed record ResolvedConfiguration(
    IReadOnlyList<string> EnvironmentVariables,
    IReadOnlyList<ResolvedConfigurationEntry> Entries,
    IReadOnlyList<string> RedactionValues,
    int VariableCount,
    int SecretCount);

internal sealed record ResolvedConfigurationEntry(
    string Name,
    ConfigurationEntryKind Kind,
    string Value,
    string? SecretName = null);
