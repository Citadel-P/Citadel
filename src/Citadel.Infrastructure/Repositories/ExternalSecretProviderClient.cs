using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Infrastructure.HttpClients.Serializer;
using Infrastructure.Vault;
using Refit;
using System.Text.Json;

namespace Infrastructure.Repositories;

internal sealed class ExternalSecretProviderClient(IHttpClientFactory httpClientFactory) : IExternalSecretProviderClient
{
    internal const string HttpClientName = "VaultKvV2";

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
            var httpClient = httpClientFactory.CreateClient(HttpClientName);
            httpClient.BaseAddress = new Uri(provider.Configuration.Address.TrimEnd('/'));
            var api = RestService.For<IVaultKvV2Api>(
                httpClient,
                new RefitSettings { ContentSerializer = new STJSourceGeneratorSerializer() });

            var response = await api.ReadSecretAsync(
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
            return ExternalSecretValueResult.Failure(
                $"Secret {secret.Name} could not be resolved from provider {provider.Name}: HTTP {(int)ex.StatusCode}.");
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException)
        {
            return ExternalSecretValueResult.Failure(
                $"Secret {secret.Name} could not be resolved from provider {provider.Name}: {ex.Message}");
        }
    }
}
