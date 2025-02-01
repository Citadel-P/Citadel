using System.Collections.Concurrent;
using Grpc.Net.Client;
using Infrastructure.Services.Abstractions;
using static Agent.Server.Containers.Containers;
using static Agent.Server.GPlatform.gPlatform;

namespace Infrastructure.Services;

internal class GrpcClientFactory : IGrpcClientFactory
{
    private readonly ConcurrentDictionary<string, gPlatformClient> platformChannels = [];
    private readonly ConcurrentDictionary<string, ContainersClient> containerChannels = [];

    public gPlatformClient GetPlatformClient(string address)
    {
        var normalizedAddress = NormalizeAddress(address);
        if (platformChannels.TryGetValue(normalizedAddress, out gPlatformClient value))
            return value;

        var channel = GrpcChannel.ForAddress(normalizedAddress);
        var client = new gPlatformClient(channel);
        platformChannels.TryAdd(normalizedAddress, client);
        return client;
    }

    public ContainersClient GetContainerClient(string address)
    {
        var normalizedAddress = NormalizeAddress(address);
        if (containerChannels.TryGetValue(normalizedAddress, out ContainersClient value))
            return value;

        var channel = GrpcChannel.ForAddress(normalizedAddress);
        var client = new ContainersClient(channel);
        containerChannels.TryAdd(normalizedAddress, client);
        return client;
    }

    private static string NormalizeAddress(string address)
    {
        if (address.StartsWith("http://") || address.StartsWith("https://"))
            return address;
        return $"http://{address}";
    }
}
