using Refit;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Infrastructure.Vault;

internal interface IVaultKvV2Api
{
    [Get("/v1/{mountPath}/data/{**secretPath}")]
    Task<VaultKvV2ReadResponse> ReadSecretAsync(
        string mountPath,
        string secretPath,
        [Header("X-Vault-Token")] string token,
        [AliasAs("version")] int? version = null,
        CancellationToken cancellationToken = default);
}

internal sealed record VaultKvV2ReadResponse(
    [property: JsonPropertyName("data")] VaultKvV2DataEnvelope Data);

internal sealed record VaultKvV2DataEnvelope(
    [property: JsonPropertyName("data")] IReadOnlyDictionary<string, JsonElement> Data);
