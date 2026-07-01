using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Infrastructure.Vault;
using Refit;
using System.Text.Json;

namespace Infrastructure.Repositories;

internal sealed class ExternalSecretProviderClient(IVaultKvV2ApiFactory vaultApiFactory) : IExternalSecretProviderClient
{
    public async Task<ExternalSecretProviderConnectionTestResult> TestConnectionAsync(
        SecretProvider provider,
        string token,
        CancellationToken cancellationToken)
    {
        if (provider.ProviderType != SecretProviderType.VaultCompatibleKvV2)
            return ExternalSecretProviderConnectionTestResult.Failed($"Secret provider {provider.ProviderType} is not implemented.");

        if (string.IsNullOrWhiteSpace(token))
            return ExternalSecretProviderConnectionTestResult.Failed($"Secret provider {provider.Name} token is empty.");

        try
        {
            var providerAddress = provider.Configuration.Address.TrimEnd('/');
            var vaultApi = vaultApiFactory.Create(providerAddress);

            var healthResponse = await vaultApi.GetHealthAsync(cancellationToken);
            var healthStatus = (int)healthResponse.StatusCode;
            var vaultHealthRecognized = healthStatus is 200 or 429 or 472 or 473;
            if (healthStatus is 501 or 503)
                return ExternalSecretProviderConnectionTestResult.Failed(
                    $"Vault endpoint is reachable but not ready: HTTP {healthStatus}.");

            if (healthStatus >= 500)
                return ExternalSecretProviderConnectionTestResult.Failed(
                    $"Vault endpoint returned HTTP {healthStatus}.");

            var tokenResponse = await vaultApi.LookupSelfAsync(token, cancellationToken);

            if (tokenResponse.IsSuccessStatusCode)
                return ExternalSecretProviderConnectionTestResult.Succeeded(
                    "Connection successful. Vault is reachable and the token is valid.");

            if (tokenResponse.StatusCode is System.Net.HttpStatusCode.NotFound or System.Net.HttpStatusCode.MethodNotAllowed)
            {
                return vaultHealthRecognized
                    ? ExternalSecretProviderConnectionTestResult.Succeeded(
                        "Vault is reachable. Token lookup is not supported by this provider; test a secret reference to verify token and KV access.")
                    : ExternalSecretProviderConnectionTestResult.Failed(
                        "Endpoint is reachable, but Vault health and token lookup endpoints were not available. Test a secret reference to verify this provider.");
            }

            if (tokenResponse.StatusCode is System.Net.HttpStatusCode.Forbidden or System.Net.HttpStatusCode.Unauthorized)
                return ExternalSecretProviderConnectionTestResult.Failed(
                    $"Vault is reachable but the token was rejected: HTTP {(int)tokenResponse.StatusCode}.");

            return ExternalSecretProviderConnectionTestResult.Failed(
                $"Vault token lookup returned HTTP {(int)tokenResponse.StatusCode}.");
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException)
        {
            return ExternalSecretProviderConnectionTestResult.Failed(
                $"Vault endpoint could not be reached: {ex.Message}");
        }
    }

    public async Task<ExternalSecretValueResult> ResolveAsync(
        SecretDefinition secret,
        SecretProvider provider,
        string token,
        CancellationToken cancellationToken)
    {
        if (provider.ProviderType != SecretProviderType.VaultCompatibleKvV2 ||
            secret.ProviderType != SecretProviderType.VaultCompatibleKvV2)
            return ExternalSecretValueResult.Failure($"Secret provider {provider.ProviderType} is not implemented.");

        if (string.IsNullOrWhiteSpace(token))
            return ExternalSecretValueResult.Failure($"Secret provider {provider.Name} token is empty.");

        try
        {
            var vaultApi = vaultApiFactory.Create(provider.Configuration.Address);
            var response = await vaultApi.ReadSecretAsync(
                provider.Configuration.MountPath.Trim('/'),
                secret.ExternalPath!.Trim('/'),
                token,
                secret.ExternalVersion,
                cancellationToken);

            if (response.Data?.Data is null)
                return ExternalSecretValueResult.Failure(
                    $"Secret {secret.Name} response from provider {provider.Name} did not contain a KV v2 data payload.");

            if (!response.Data.Data.TryGetValue(secret.ExternalKey!, out var value))
                return ExternalSecretValueResult.Failure($"Secret {secret.Name} key '{secret.ExternalKey}' was not found in provider {provider.Name}.");

            return ExternalSecretValueResult.Success(value.ValueKind == JsonValueKind.String
                ? value.GetString() ?? string.Empty
                : value.GetRawText());
        }
        catch (ApiException ex)
        {
            if (ex.StatusCode == System.Net.HttpStatusCode.NotFound)
            {
                return ExternalSecretValueResult.Failure(
                    $"Secret {secret.Name} was not found in provider {provider.Name} at KV v2 path '{FormatKvV2Path(provider, secret)}'. " +
                    "Check that the provider mount path is only the KV engine mount, and the secret path is relative to that mount without '/data'.");
            }

            return ExternalSecretValueResult.Failure(
                $"Secret {secret.Name} could not be resolved from provider {provider.Name}: HTTP {(int)ex.StatusCode}.");
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException)
        {
            return ExternalSecretValueResult.Failure(
                $"Secret {secret.Name} could not be resolved from provider {provider.Name}: {ex.Message}");
        }
    }

    private static string FormatKvV2Path(SecretProvider provider, SecretDefinition secret)
        => $"{provider.Configuration.MountPath.Trim('/')}/data/{secret.ExternalPath!.Trim('/')}";
}
