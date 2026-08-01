using Infrastructure.HttpClients.Serializer;
using Refit;

namespace Infrastructure.Vault;

internal interface IVaultKvV2ApiFactory
{
    IVaultKvV2Api Create(string providerAddress);
}

internal sealed class VaultKvV2ApiFactory(IHttpClientFactory httpClientFactory) : IVaultKvV2ApiFactory
{
    private readonly RefitSettings settings = new()
    {
        ContentSerializer = new STJSourceGeneratorSerializer()
    };

    public IVaultKvV2Api Create(string providerAddress)
    {
        var client = httpClientFactory.CreateClient("VaultKvV2");
        client.BaseAddress = new Uri(providerAddress.TrimEnd('/'));
        return RestService.For<IVaultKvV2Api>(client, settings);
    }
}
