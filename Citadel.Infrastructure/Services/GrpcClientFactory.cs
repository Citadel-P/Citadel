using System.Collections.Concurrent;
using Grpc.Net.Client;
using Infrastructure.Services.Abstractions;
using static Agent.Server.Containers.Containers;
using static Agent.Server.GPlatform.gPlatform;
using static Agent.Server.Images.Images;
using static Agent.Server.Networks.Networks;

namespace Infrastructure.Services;

internal class GrpcClientFactory : IGrpcClientFactory
{
    private readonly ConcurrentDictionary<string, GrpcChannel> _channelCache = new();
    private readonly ConcurrentDictionary<(Type, string), object> _clientCache = new();

    public gPlatformClient GetPlatformClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new gPlatformClient(channel));

    public ContainersClient GetContainerClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new ContainersClient(channel));

    public ImagesClient GetImageClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new ImagesClient(channel));

    public NetworksClient GetNetworkClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new NetworksClient(channel));

    private TClient GetOrCreateClient<TClient>(string address, Func<GrpcChannel, TClient> factory)
    {
        // Cache channel per address
        var channel = _channelCache.GetOrAdd(address, GrpcChannel.ForAddress);

        // Combine client type + address as cache key
        var key = (typeof(TClient), address);
        if (_clientCache.TryGetValue(key, out var cached))
            return (TClient)cached;

        var client = factory(channel);
        _clientCache.TryAdd(key, client);
        return client;
    }

    private static string NormalizeAddress(string address) =>
        address.StartsWith("http://") || address.StartsWith("https://")
            ? address
            : $"http://{address}";
}
