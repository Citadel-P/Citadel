using Application.Configs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Entities.ResourceBindings;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;

namespace Application.Services;

internal interface IResourceBindingResolver
{
    Task<Result<ResolvedResourceBindings>> ResolveAsync(
        ResourceBindingScope scope,
        Guid resourceId,
        CancellationToken cancellationToken);

    Task<Result<ResolvedResourceBindings>> ResolveWithoutMountedSecretsAsync(
        ResourceBindingScope scope,
        Guid resourceId,
        CancellationToken cancellationToken)
        => ResolveAsync(scope, resourceId, cancellationToken);
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

internal sealed partial class ResourceBindingResolver(
    IServiceScopeFactory scopeFactory,
    ISecretValueProtector secretValueProtector,
    IExternalSecretProviderClient externalSecretProviderClient) : IResourceBindingResolver
{
    private static readonly Regex NameRegex = GetNameRegex();

    public Task<Result<ResolvedResourceBindings>> ResolveAsync(
        ResourceBindingScope scope,
        Guid resourceId,
        CancellationToken cancellationToken)
        => ResolveCoreAsync(scope, resourceId, resolveMountedSecrets: true, cancellationToken);

    public Task<Result<ResolvedResourceBindings>> ResolveWithoutMountedSecretsAsync(
        ResourceBindingScope scope,
        Guid resourceId,
        CancellationToken cancellationToken)
        => ResolveCoreAsync(scope, resourceId, resolveMountedSecrets: false, cancellationToken);

    private async Task<Result<ResolvedResourceBindings>> ResolveCoreAsync(
        ResourceBindingScope scope,
        Guid resourceId,
        bool resolveMountedSecrets,
        CancellationToken cancellationToken)
    {
        await using var serviceScope = scopeFactory.CreateAsyncScope();
        var uow = serviceScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var entries = await uow.ResourceBindings.GetEffectiveEntriesAsync(scope, resourceId, cancellationToken);

        var effective = MergeEffective(entries);
        var environment = new Dictionary<string, string>(StringComparer.Ordinal);
        var resolvedEntries = new List<ResolvedResourceBinding>();
        var snapshotEntries = new List<ResourceBindingSnapshot>();
        var redactionValues = new List<string>();
        var variableCount = 0;
        var secretCount = 0;

        foreach (var entry in effective)
        {
            if (!NameRegex.IsMatch(entry.Name))
                return Result.Failure<ResolvedResourceBindings>($"Configuration key '{entry.Name}' is not a valid environment variable name.");

            try
            {
                entry.Validate();
            }
            catch (ArgumentException ex)
            {
                return Result.Failure<ResolvedResourceBindings>(ex.Message);
            }

            if (entry.Kind == ResourceBindingKind.Variable)
            {
                variableCount++;
                var variableValue = entry.Value ?? string.Empty;
                environment[entry.Name] = variableValue;
                resolvedEntries.Add(new ResolvedResourceBinding(entry.Name, entry.Kind, variableValue));
                snapshotEntries.Add(entry.ToVariableSnapshot(variableValue));
                continue;
            }

            secretCount++;
            if (entry.SecretDeliveryMode == SecretDeliveryMode.NativePlatformSecret)
                return Result.Failure<ResolvedResourceBindings>(
                    $"Secret {entry.Name} uses delivery mode {entry.SecretDeliveryMode}, which is not supported yet.");

            if (entry.SecretDeliveryMode == SecretDeliveryMode.MountedFile
                && (scope != ResourceBindingScope.Stack || entry.Scope != ResourceBindingScope.Stack))
            {
                return Result.Failure<ResolvedResourceBindings>(
                    $"Secret {entry.Name} uses mounted-file delivery, which is only supported for stack-scoped entries.");
            }

            var secret = await uow.SecretDefinitions.GetAsync(entry.SecretId!.Value, cancellationToken);
            if (secret is null)
                return Result.Failure<ResolvedResourceBindings>($"Secret {entry.Name} is not available.");

            if (!resolveMountedSecrets && entry.SecretDeliveryMode == SecretDeliveryMode.MountedFile)
            {
                resolvedEntries.Add(new ResolvedResourceBinding(
                    entry.Name,
                    entry.Kind,
                    string.Empty,
                    secret.Name,
                    entry.SecretDeliveryMode,
                    entry.TargetPath));
                snapshotEntries.Add(entry.ToSecretSnapshot(secret, provider: null));
                continue;
            }

            var plaintextResult = await ResolveSecretPlaintextAsync(uow, entry.Name, secret, cancellationToken);
            if (!plaintextResult.IsSuccess(out var resolvedSecret, out var plaintextError))
                return Result.Failure<ResolvedResourceBindings>(plaintextError!);

            var plaintext = resolvedSecret.Plaintext;
            if (entry.SecretDeliveryMode == SecretDeliveryMode.EnvironmentVariable)
            {
                environment[entry.Name] = plaintext;
            }

            resolvedEntries.Add(new ResolvedResourceBinding(
                entry.Name,
                entry.Kind,
                plaintext,
                secret.Name,
                entry.SecretDeliveryMode,
                entry.TargetPath));
            snapshotEntries.Add(entry.ToSecretSnapshot(secret, resolvedSecret.Provider));
            if (!string.IsNullOrEmpty(plaintext))
            {
                redactionValues.Add(plaintext);
            }
        }

        return new ResolvedResourceBindings(
            EnvironmentVariables: [.. environment.Select(kv => $"{kv.Key}={kv.Value}")],
            Entries: resolvedEntries,
            RedactionValues: [.. redactionValues],
            VariableCount: variableCount,
            SecretCount: secretCount)
        {
            SnapshotEntries = snapshotEntries
        };
    }

    private async Task<Result<ResolvedSecretPlaintext>> ResolveSecretPlaintextAsync(
        IUnitOfWork uow,
        string entryName,
        SecretDefinition secret,
        CancellationToken cancellationToken)
    {
        if (secret.ProviderType == SecretProviderType.InternalEncrypted)
        {
            var value = await uow.SecretDefinitions.GetInternalValueAsync(secret.Id, cancellationToken);
            if (value is null)
                return Result.Failure<ResolvedSecretPlaintext>($"Secret {entryName} is not available.");

            var plaintext = UnprotectSecret(value.EncryptedValue, $"Secret {entryName} could not be decrypted.");
            return plaintext.IsSuccess(out var valuePlaintext, out var error)
                ? new ResolvedSecretPlaintext(valuePlaintext, null)
                : Result.Failure<ResolvedSecretPlaintext>(error!);
        }

        if (secret.ProviderId is null)
            return Result.Failure<ResolvedSecretPlaintext>($"Secret {entryName} does not reference a provider.");

        var provider = await uow.SecretProviders.GetAsync(secret.ProviderId.Value, cancellationToken);
        if (provider is null)
            return Result.Failure<ResolvedSecretPlaintext>($"Secret provider for {entryName} is not available.");

        var tokenResult = UnprotectSecret(
            provider.Configuration.ProtectedToken,
            $"Secret provider token for {entryName} could not be decrypted.");
        if (!tokenResult.IsSuccess(out var token, out var tokenError))
            return Result.Failure<ResolvedSecretPlaintext>(tokenError!);

        var result = await externalSecretProviderClient.ResolveAsync(secret, provider, token, cancellationToken);
        return result.IsSuccess
            ? new ResolvedSecretPlaintext(result.Value ?? string.Empty, provider)
            : Result.Failure<ResolvedSecretPlaintext>(result.ErrorMessage ?? $"Secret {entryName} could not be resolved.");
    }

    private Result<string> UnprotectSecret(string protectedValue, string failureMessage)
    {
        try
        {
            return secretValueProtector.Unprotect(protectedValue);
        }
        catch (Exception ex) when (ex is FormatException or CryptographicException)
        {
            return Result.Failure<string>(failureMessage);
        }
    }

    private static IReadOnlyList<ResourceBinding> MergeEffective(IEnumerable<ResourceBinding> entries)
    {
        var effective = new Dictionary<string, ResourceBinding>(StringComparer.Ordinal);
        foreach (var entry in entries.OrderBy(x => x.Scope == ResourceBindingScope.Global ? 0 : 1))
        {
            effective[entry.Name] = entry;
        }

        return effective.Values.OrderBy(x => x.Name, StringComparer.Ordinal).ToArray();
    }

    [GeneratedRegex("^[A-Za-z_][A-Za-z0-9_]*$", RegexOptions.Compiled)]
    private static partial Regex GetNameRegex();
}

internal sealed class SecretValueProtector(IOptions<SecretsConfiguration> secretsOptions) : ISecretValueProtector
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
        => secretsOptions.Value.GetEncryptionKey();
}

internal sealed class SecretRedactor : ISecretRedactor
{
    public string Redact(string? value, IEnumerable<string> secrets)
    {
        if (string.IsNullOrEmpty(value))
            return string.Empty;

        var redacted = value;
        foreach (var secret in secrets
            .Where(x => !string.IsNullOrEmpty(x))
            .Distinct(StringComparer.Ordinal)
            .OrderByDescending(x => x.Length))
        {
            redacted = redacted.Replace(secret, "********", StringComparison.Ordinal);
        }

        return redacted;
    }
}

internal sealed record ResolvedResourceBindings(
    IReadOnlyList<string> EnvironmentVariables,
    IReadOnlyList<ResolvedResourceBinding> Entries,
    IReadOnlyList<string> RedactionValues,
    int VariableCount,
    int SecretCount)
{
    public IReadOnlyList<ResourceBindingSnapshot> SnapshotEntries { get; init; } = [];

    public IReadOnlyDictionary<string, string> ToValueDictionary()
        => Entries.ToDictionary(x => x.Name, x => x.Value, StringComparer.Ordinal);

    public ResolvedResourceBindings SelectEntries(IEnumerable<string> names)
    {
        var selectedNames = names.ToHashSet(StringComparer.Ordinal);
        var selectedEntries = Entries.Where(entry => selectedNames.Contains(entry.Name)).ToArray();

        return this with
        {
            Entries = selectedEntries,
            EnvironmentVariables =
            [
                .. selectedEntries
                    .Where(entry => entry.Kind == ResourceBindingKind.Variable
                        || entry.SecretDeliveryMode == SecretDeliveryMode.EnvironmentVariable)
                    .Select(entry => $"{entry.Name}={entry.Value}")
            ],
            RedactionValues =
            [
                .. selectedEntries
                    .Where(entry => entry.Kind == ResourceBindingKind.Secret)
                    .Select(entry => entry.Value)
                    .Where(value => !string.IsNullOrEmpty(value))
            ],
            SnapshotEntries = [.. SnapshotEntries.Where(entry => selectedNames.Contains(entry.Name))],
            VariableCount = selectedEntries.Count(entry => entry.Kind == ResourceBindingKind.Variable),
            SecretCount = selectedEntries.Count(entry => entry.Kind == ResourceBindingKind.Secret)
        };
    }
}

internal sealed record ResolvedResourceBinding(
    string Name,
    ResourceBindingKind Kind,
    string Value,
    string? SecretName = null,
    SecretDeliveryMode? SecretDeliveryMode = null,
    string? TargetPath = null);

internal sealed record ResolvedSecretPlaintext(string Plaintext, SecretProvider? Provider);
