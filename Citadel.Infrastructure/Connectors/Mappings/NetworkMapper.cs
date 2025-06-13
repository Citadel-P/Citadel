using Citadel.Agent.Networks.V1;
using Domain.Contracts.Resources.Networks;
using Google.Protobuf.Collections;

namespace Infrastructure.Connectors.Mappings;

public static class NetworkMapper
{
    public static IEnumerable<DockerNetwork> Map(this ListNetworksResponse response)
        => response.Networks.Select(Map);

    public static IEnumerable<DockerNetwork> Map(this RepeatedField<Network> networks)
        => networks.Select(Map);

    public static DockerNetwork Map(this Network network)
        => new 
        (
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
            InUse: network.InUse,
            ConfigFrom: network.ConfigFrom,
            Ipam: network.Ipam?.Map(),
            Options: network.Options ?? [],
            Labels: network.Labels
        );

    public static IpAddressManagementConfig Map(this IPAMMessage Ipam)
        => new
        (
            Driver: Ipam?.Driver,
            Config: Ipam?.Config.Select(c => new IpamSubnetConfiguration(
                Subnet: c.Subnet,
                IpRange: c.IpRange,
                Gateway: c.Gateway)).ToList() ?? [],
            Options: Ipam?.Options ?? []
        );

    public static DockerNetworkDetails Map(this InspectNetworkResponse response)
        => new
        (
            Name: response.Name,
            Id: response.Id,
            Created: response.Created,
            Driver: response.Driver,
            Scope: response.Scope,
            EnableIPv4: response.EnableIPv4,
            EnableIPv6: response.EnableIPv6,
            Internal: response.Internal,
            Attachable: response.Attachable,
            Ingress: response.Ingress,
            ConfigOnly: response.ConfigOnly,
            InUse: response.InUse,
            ConfigFrom: response.ConfigFrom,
            Ipam: response.Ipam?.Map(),
            Options: response.Options ?? [],
            Labels: response.Labels ?? [],
            Containers: response.Containers?.ToDictionary(c => c.Key, c => c.Value.Map()) ?? [],
            Peers: response.Peers.Select(p => new NetworkPeerInfo(p.Name, p.Ip))?.ToList() ?? []
        );

    public static NetworkConnectedContainer Map(this NetworkContainerMessage container)
        => new 
        (
            Name: container.Name,
            EndpointId: container.EndpointId,
            MacAddress: container.MacAddress,
            Ipv6Address: container.Ipv6Address,
            IpV4Address: container.IpPv4Address
        );
}
