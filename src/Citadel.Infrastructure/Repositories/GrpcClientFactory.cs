using System.Collections.Concurrent;
using Grpc.Core;
using Grpc.Core.Interceptors;
using Grpc.Net.Client;
using Grpc.Net.Client.Configuration;
using static Citadel.Agent.Containers.V1.ContainerService;
using static Citadel.Agent.Images.V1.ImageService;
using static Citadel.Agent.Networks.V1.NetworkService;
using static Citadel.Agent.Platforms.V1.PlatformService;
using static Citadel.Agent.Volumes.V1.VolumeService;
using static Citadel.Agent.Deployments.V1.DeploymentService;

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
    DeploymentServiceClient GetDeploymentClient(string address);
}

internal class GrpcClientFactory(params Interceptor[] interceptors) : IGrpcClientFactory
{
    private readonly ConcurrentDictionary<string, GrpcChannel> _channelCache = new();
    private readonly ConcurrentDictionary<(Type, string), object> _clientCache = new();
    private readonly Interceptor[] _interceptors = interceptors ?? [];

    public PlatformServiceClient GetPlatformClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), invoker => new PlatformServiceClient(invoker));

    public ContainerServiceClient GetContainerClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), invoker => new ContainerServiceClient(invoker));

    public ImageServiceClient GetImageClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), invoker => new ImageServiceClient(invoker));

    public NetworkServiceClient GetNetworkClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), invoker => new NetworkServiceClient(invoker));

    public VolumeServiceClient GetVolumeClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), invoker => new VolumeServiceClient(invoker));

    public DeploymentServiceClient GetDeploymentClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), invoker => new DeploymentServiceClient(invoker));

    private TClient GetOrCreateClient<TClient>(string address, Func<CallInvoker, TClient> factory)
    {
        // Cache channel per address
        var channel = _channelCache.GetOrAdd(address, CreateChannel);

        // Wrap channel in CallInvoker with interceptors
        CallInvoker invoker = channel.CreateCallInvoker();
        if (_interceptors.Length > 0)
            invoker = invoker.Intercept(_interceptors);

        // Combine client type + address as cache key
        var key = (typeof(TClient), address);
        return (TClient)_clientCache.GetOrAdd(key, _ => factory(invoker));
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
                            RetryableStatusCodes = { StatusCode.Unavailable }
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
