using System.Collections.Concurrent;
using Domain.Configs;
using Domain.Contracts.Interfaces;
using Grpc.Core;
using Grpc.Core.Interceptors;
using Grpc.Net.Client;
using Grpc.Net.Client.Configuration;
using Hosting.Common.Security;
using Infrastructure.EdgeAgents;
using Microsoft.Extensions.Options;
using static Citadel.Containers.V1.ContainerService;
using static Citadel.Images.V1.ImageService;
using static Citadel.Networks.V1.NetworkService;
using static Citadel.Platforms.V1.PlatformService;
using static Citadel.Volumes.V1.VolumeService;
using static Citadel.Deployments.V1.DeploymentService;
using static Citadel.Stacks.V1.StackService;

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
    StackServiceClient GetStackClient(string address);
}

internal sealed class GrpcClientFactory(
    IOptions<AgentTransportOptions> transportOptions,
    CertificateTrust certificateTrust,
    params Interceptor[] interceptors)
    : IGrpcClientFactory, IPlatformConnectionCache, IDisposable
{
    private readonly ConcurrentDictionary<string, GrpcChannel> _channelCache = new();
    private readonly ConcurrentDictionary<(Type, string), object> _clientCache = new();
    private readonly Interceptor[] _interceptors = interceptors ?? [];
    private readonly AgentTransportOptions _transportOptions = transportOptions.Value;
    private readonly CertificateTrust _certificateTrust = certificateTrust;
    private readonly Lock _gate = new();

    internal int ChannelCount => _channelCache.Count;
    internal int ClientCount => _clientCache.Count;

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

    public StackServiceClient GetStackClient(string address) =>
        GetOrCreateClient(NormalizeAddress(address), invoker => new StackServiceClient(invoker));

    private TClient GetOrCreateClient<TClient>(string address, Func<CallInvoker, TClient> factory)
    {
        var key = (typeof(TClient), address);
        if (_clientCache.TryGetValue(key, out var cachedClient))
        {
            return (TClient)cachedClient;
        }

        using (_gate.EnterScope())
        {
            if (_clientCache.TryGetValue(key, out cachedClient))
            {
                return (TClient)cachedClient;
            }

            var channel = _channelCache.GetOrAdd(address, CreateChannel);

            CallInvoker invoker = channel.CreateCallInvoker();
            if (_interceptors.Length > 0)
                invoker = invoker.Intercept(_interceptors);

            var client = factory(invoker);
            _clientCache[key] = client!;
            return client;
        }
    }

    public void Evict(string address)
    {
        GrpcChannel? channel = null;
        var normalizedAddress = NormalizeAddress(address);

        using (_gate.EnterScope())
        {
            foreach (var key in _clientCache.Keys)
            {
                if (string.Equals(key.Item2, normalizedAddress, StringComparison.Ordinal))
                    _clientCache.TryRemove(key, out _);
            }

            _channelCache.TryRemove(normalizedAddress, out channel);
        }

        channel?.Dispose();
    }

    public void Dispose()
    {
        GrpcChannel[] channels;
        using (_gate.EnterScope())
        {
            channels = [.. _channelCache.Values];
            _clientCache.Clear();
            _channelCache.Clear();
        }

        foreach (var channel in channels)
            channel.Dispose();
    }

    private GrpcChannel CreateChannel(string address)
    {
        var uri = new Uri(address, UriKind.Absolute);
        var handler = uri.Scheme == Uri.UriSchemeHttps
            ? new SocketsHttpHandler
            {
                EnableMultipleHttp2Connections = true
            }
            : null;
        if (handler is not null)
        {
            _certificateTrust.Configure(handler);
        }

        return GrpcChannel.ForAddress(address, new GrpcChannelOptions
        {
            HttpHandler = handler,
            MaxReceiveMessageSize = EdgeAgentDefaults.MaxEnvelopePayloadBytes,
            MaxSendMessageSize = EdgeAgentDefaults.MaxEnvelopePayloadBytes,
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

    private string NormalizeAddress(string address)
    {
        if (string.IsNullOrWhiteSpace(address))
            throw new ArgumentException("gRPC address must not be null or empty.", nameof(address));

        var candidate = address.Trim();
        if (_channelCache.ContainsKey(candidate))
        {
            return candidate;
        }

        if (!Uri.TryCreate(candidate, UriKind.Absolute, out var uri)
            || string.IsNullOrWhiteSpace(uri.Host)
            || (uri.Scheme != Uri.UriSchemeHttp
                && uri.Scheme != Uri.UriSchemeHttps)
            || !string.IsNullOrEmpty(uri.UserInfo)
            || !string.IsNullOrEmpty(uri.Query)
            || !string.IsNullOrEmpty(uri.Fragment)
            || uri.AbsolutePath != "/")
        {
            throw new ArgumentException(
                "gRPC address must be an absolute HTTP or HTTPS origin without credentials, path, query, or fragment.",
                nameof(address));
        }

        if (!_transportOptions.AllowInsecure
            && uri.Scheme != Uri.UriSchemeHttps)
        {
            throw new InvalidOperationException(
                "The Agent endpoint must use HTTPS because insecure Agent transport is disabled.");
        }

        return uri.AbsoluteUri.TrimEnd('/');
    }
}
