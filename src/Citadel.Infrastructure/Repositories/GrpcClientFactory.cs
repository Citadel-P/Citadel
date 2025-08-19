using System.Collections.Concurrent;
using Grpc.Core;
using Grpc.Net.Client;
using Grpc.Net.Client.Configuration;
using static Citadel.Agent.Containers.V1.ContainerService;
using static Citadel.Agent.Images.V1.ImageService;
using static Citadel.Agent.Networks.V1.NetworkService;
using static Citadel.Agent.Platforms.V1.PlatformService;
using static Citadel.Agent.Volumes.V1.VolumeService;

namespace Infrastructure.Repositories;

/// <summary>
/// Factory to create gRPC clients (native gRPC factory does not support address change at runtime; see https://github.com/grpc/grpc-dotnet/issues/1641)
/// </summary>
internal interface IGrpcClientFactory
{
    PlatformServiceClient GetPlatformClient(string address);
    ContainerServiceClient GetContainerClient(string address);
    ImageServiceClient GetImageClient(string address);
    NetworkServiceClient GetNetworkClient(string address);
    VolumeServiceClient GetVolumeClient(string address);
}

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
        var channel = _channelCache.GetOrAdd(address, CreateChannel);

        // Combine client type + address as cache key
        var key = (typeof(TClient), address);
        return (TClient)_clientCache.GetOrAdd(key, _ => factory(channel));
    }

    private static GrpcChannel CreateChannel(string address)
    {
        return GrpcChannel.ForAddress(address, new GrpcChannelOptions
        {
            ServiceConfig = new ServiceConfig
            {
                MethodConfigs =
                {
                    new MethodConfig
                    {
                        Names = { MethodName.Default }, // applies to all methods
                        RetryPolicy = new RetryPolicy
                        {
                            MaxAttempts = 3,
                            InitialBackoff = TimeSpan.FromMilliseconds(200),
                            MaxBackoff = TimeSpan.FromSeconds(2),
                            BackoffMultiplier = 2,
                            RetryableStatusCodes =
                            {
                                StatusCode.Unavailable // network/transient failures
                            }
                        }
                    }
                }
            }
        });
    }

    private static string NormalizeAddress(string address)
    {
        if (string.IsNullOrWhiteSpace(address))
            throw new ArgumentException("gRPC address must not be null or empty.", nameof(address));

        if (address.StartsWith("http://", StringComparison.OrdinalIgnoreCase) ||
            address.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
            return address;

        return $"http://{address}";
    }
}
