using Agent.Server.Networks;

namespace WebApi.Routes.Endpoints.Resources.Networks;

public sealed record InspectNetworkView(
    string Name,
    string Id,
    string Created,
    string Driver,
    string Scope,
    bool? EnableIPv4,
    bool? EnableIPv6,
    bool? Internal,
    bool? Attachable,
    bool? Ingress,
    bool? InUse,
    bool? ConfigOnly,
    string? ConfigFrom = null,
    IPAMView? Ipam = null,
    IEnumerable<PeerInfoView>? Peers = null,
    Dictionary<string, string>? Options = null,
    Dictionary<string, string>? Labels = null,
    Dictionary<string, NetworkContainerView>? Containers = null
    )
{
    public static InspectNetworkView Map(InspectNetworkReply network) 
        => new (
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
            InUse: network.Containers?.Count > 0,
            ConfigOnly: network.ConfigOnly,
            ConfigFrom: network.ConfigFrom,
            network.Ipam != null ? new IPAMView(network.Ipam.Driver, IPAMConfigView.Map(network.Ipam.Config?.ToList() ?? []), network.Ipam.Options?.ToDictionary() ?? []) : null,
            Options: network.Options?.ToDictionary(),
            Labels: network.Labels?.ToDictionary(),
            Peers: network.Peers?.Select(PeerInfoView.Map),
            Containers: network.Containers?.ToDictionary(
                x => x.Key,
                x => NetworkContainerView.Map(x.Value)
            )
        );
}


public sealed record NetworkContainerView(
    string Name,
    string EndpointId,
    string MacAddress,
    string IPv4Address,
    string IPv6Address
)
{
    public static NetworkContainerView Map(NetworkContainerMessage container) 
        => new (
            Name: container.Name,
            EndpointId: container.EndpointId,
            MacAddress: container.MacAddress,
            IPv4Address: container.IpPv4Address,
            IPv6Address: container.Ipv6Address
        );
}

public sealed record PeerInfoView(string Name , string Ip)
{
       public static PeerInfoView Map(PeerInfoMessage peer) 
        => new (
            Name: peer.Name,
            Ip: peer.Ip
        );
}