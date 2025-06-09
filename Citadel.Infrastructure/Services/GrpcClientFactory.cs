using System.Collections.Concurrent;
using Grpc.Net.Client;
using Infrastructure.Services.Abstractions;
using static Citadel.Agent.Containers.V1.ContainerService;
using static Citadel.Agent.Platforms.V1.PlatformService;
using static Citadel.Agent.Images.V1.ImageService;
using static Citadel.Agent.Networks.V1.NetworkService;
using static Citadel.Agent.Volumes.V1.VolumeService;

namespace Infrastructure.Services;

internal class GrpcClientFactory : IGrpcClientFactory
{
    private readonly ConcurrentDictionary<string, GrpcChannel> _channelCache = new();
    private readonly ConcurrentDictionary<(Type, string), object> _clientCache = new();

    public PlatformServiceClient GetPlatformClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new PlatformServiceClient(channel));

    public ContainerServiceClient GetContainerClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new ContainerServiceClient(channel));

    public ImageServiceClient GetImageClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new ImageServiceClient(channel));

    public NetworkServiceClient GetNetworkClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new NetworkServiceClient(channel));

    public VolumeServiceClient GetVolumeClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), channel => new VolumeServiceClient(channel));

    private TClient GetOrCreateClient<TClient>(string address, Func<GrpcChannel, TClient> factory)
    {
        // Cache channel per address
        var channel = _channelCache.GetOrAdd(address, GrpcChannel.ForAddress);

        // Combine client type + address as cache key
        var key = (typeof(TClient), address);
        return (TClient)_clientCache.GetOrAdd(key, _ => factory(channel));
    }

    private static string NormalizeAddress(string address)
    {
        if (string.IsNullOrWhiteSpace(address))
            throw new ArgumentException("gRPC address must not be null or empty.", nameof(address));

        if (address.StartsWith("http://", StringComparison.OrdinalIgnoreCase) ||
            address.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
            return address;

        // Default to http
        return $"http://{address}";
    }
}
