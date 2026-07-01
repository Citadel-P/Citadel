using Hosting.DockerClient;
using Infrastructure.HttpClients.Serializer;
using Microsoft.Extensions.Http;
using Refit;
using System.Collections.Concurrent;

namespace Infrastructure.Vault;

internal interface IVaultKvV2ApiFactory
{
    IVaultKvV2Api Create(string providerAddress);
}

internal sealed class VaultKvV2ApiFactory : IVaultKvV2ApiFactory
{
    private readonly ConcurrentDictionary<string, IVaultKvV2Api> clients = new(StringComparer.OrdinalIgnoreCase);
    private readonly RefitSettings settings = new()
    {
        ContentSerializer = new STJSourceGeneratorSerializer()
    };

    public IVaultKvV2Api Create(string providerAddress)
        => clients.GetOrAdd(providerAddress.TrimEnd('/'), CreateClient);

    private IVaultKvV2Api CreateClient(string providerAddress)
        => RestService.For<IVaultKvV2Api>(
            new HttpClient(CreateHandler())
            {
                BaseAddress = new Uri(providerAddress)
            },
            settings);

    private static HttpMessageHandler CreateHandler()
        => new PolicyHttpMessageHandler(Configuration.GetRetryPolicy())
        {
            InnerHandler = new SocketsHttpHandler()
        };
}
