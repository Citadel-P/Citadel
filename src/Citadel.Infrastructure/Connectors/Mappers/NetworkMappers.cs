using Citadel.Agent.Networks.V1;
using Domain.Contracts.Resources.Networks;
using Google.Protobuf.Collections;
using Hosting.DockerClient.Models.Networks;

namespace Infrastructure.Connectors.Mappers;

internal static class NetworkMappers
{
    public static IEnumerable<DockerNetworkResult> Map(this ListNetworksResponse networks)
        => networks.Networks.Select(Map);

    public static IEnumerable<DockerNetworkResult> Map(this RepeatedField<Network> networks)
        => networks.Select(Map);

    public static DockerNetworkResult Map(this Network network)
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

    public static IEnumerable<DockerNetworkResult> Map(this IEnumerable<NetworkResult> networks)
        => networks.Select(Map);

    public static DockerNetworkResult Map(this NetworkResult network)
        => new
        (
            Name: network.Name,
            Id: network.Id,
            Driver: network.Driver,
            Scope: network.Scope,
            Created: network.CreatedAt,
            EnableIPv4: network.EnableIPv4,
            EnableIPv6: network.EnableIPv6,
            Internal: network.Internal,
            Attachable: network.Attachable,
            Ingress: network.Ingress,
            ConfigOnly: network.ConfigOnly,
            InUse: network.InUse,
            ConfigFrom: network.ConfigFromNetworkName,
            Ipam: network.Ipam?.Map(),
            Options: network.Options?.ToDictionary() ?? [],
            Labels: network.Labels?.ToDictionary() ?? []
        );

    public static IpAddressManagementConfig Map(this Hosting.DockerClient.IPAM Ipam)
        => new
        (
            Driver: Ipam?.Driver,
            Config: Ipam?.Config?.Select(c => new IpamSubnetConfiguration(
                Subnet: c.Subnet,
                IpRange: c.IPRange,
                Gateway: c.Gateway)).ToList() ?? [],
            Options: Ipam?.Options?.ToDictionary() ?? []
        );

    public static DockerNetworkDetails Map(this InspectNetworkResult network)
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
            ConfigFrom: network.ConfigFromNetworkName,
            Ipam: network.Ipam?.Map(),
            Options: network.Options?.ToDictionary() ?? [],
            Labels: network.Labels?.ToDictionary() ?? [],
            Containers: network.Containers?.ToDictionary(c => c.Key, c => c.Value.Map()) ?? [],
            Peers: network.Peers?.Select(p => new NetworkPeerInfo(Name: p.Name, Ip: p.IP))?.ToList() ?? []
        );

    public static IpAddressManagementConfig Map(this IPAMMessage Ipam)
        => new
        (
            Driver: Ipam?.Driver,
            Config: Ipam?.Config?.Select(c => new IpamSubnetConfiguration(
                Subnet: c.Subnet,
                IpRange: c.IpRange,
                Gateway: c.Gateway)).ToList() ?? [],
            Options: Ipam?.Options ?? []
        );

    public static DockerNetworkDetails Map(this InspectNetworkResponse network)
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
            ConfigFrom: network.ConfigFrom,
            Ipam: network.Ipam?.Map(),
            Options: network.Options ?? [],
            Labels: network.Labels ?? [],
            Containers: network.Containers?.ToDictionary(c => c.Key, c => c.Value.Map()) ?? [],
            Peers: network.Peers?.Select(p => new NetworkPeerInfo(p.Name, p.Ip))?.ToList() ?? []
        );

    private static NetworkConnectedContainer Map(this Hosting.DockerClient.NetworkContainer networkContainer)
        => new
        (
            Name: networkContainer.Name,
            EndpointId: networkContainer.EndpointID,
            MacAddress: networkContainer.MacAddress,
            Ipv6Address: networkContainer.IPv6Address,
            IpV4Address: networkContainer.IPv4Address
        );

    private static NetworkConnectedContainer Map(this NetworkContainerMessage networkContainer)
        => new 
        (
            Name: networkContainer.Name,
            EndpointId: networkContainer.EndpointId,
            MacAddress: networkContainer.MacAddress,
            Ipv6Address: networkContainer.Ipv6Address,
            IpV4Address: networkContainer.IpPv4Address
        );

    public static CreateDockerNetworkResult Map(this CreateNetworkResult network)
        => new (NetworkId: network.Id);

    public static Hosting.DockerClient.IPAM Map(this IpAddressManagementConfig ipam) 
        => new()
        {
            Driver = ipam.Driver,
            Config = ipam.Config?.Select(c => new Hosting.DockerClient.IPAMConfig
            {
                Subnet = c.Subnet,
                IPRange = c.IpRange,
                Gateway = c.Gateway
            }).ToList(),
            Options = ipam.Options?.ToDictionary() ?? []
        };
}
