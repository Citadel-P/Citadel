using Application.Permissions;
using Domain.Contracts.Resources.Networks;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;

public sealed record DockerNetworkDetailsView(
    string Name,
    string Id,
    string Created,
    string Driver,
    string Scope,
    bool EnableIPv4,
    bool EnableIPv6,
    bool Internal,
    bool Attachable,
    bool Ingress,
    bool ConfigOnly,
    string? ConfigFrom,
    IpAddressManagementConfig? Ipam,
    IReadOnlyDictionary<string, string> Options,
    IReadOnlyDictionary<string, string> Labels,
    IReadOnlyDictionary<string, NetworkConnectedContainer> Containers,
    IReadOnlyList<NetworkPeerInfo> Peers,
    NetworkCapabilities? Capabilities = null
    )
{
    internal static async Task<DockerNetworkDetailsView> Map(DockerNetworkDetails netowrk, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(netowrk.PlatformId, ResourceType.Platform);
        return Map(netowrk) with
        {
            Capabilities = CapabilityMapper.ToNetworkCapabilities(permissions)
        };
    }

    private static DockerNetworkDetailsView Map(DockerNetworkDetails network)
        => new(
            Name: network.Name,
            Id: network.Id,
            Created: network.Created,
            Driver: network.Driver,
            Scope: network.Scope,
            EnableIPv4: network.EnableIPv4,
            EnableIPv6: network.EnableIPv6,
            Internal: network.Internal,
            Attachable: network.Attachable,
            Ingress: network.Ingress,
            ConfigOnly: network.ConfigOnly,
            ConfigFrom: network.ConfigFrom,
            Ipam: network.Ipam,
            Options: network.Options ?? new Dictionary<string, string>(),
            Labels: network.Labels ?? new Dictionary<string, string>(),
            Containers: network.Containers ?? new Dictionary<string, NetworkConnectedContainer>(),
            Peers: network.Peers ?? []);
}