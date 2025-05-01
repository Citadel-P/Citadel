using Agent.Server.Networks;

namespace WebApi.Routes.Endpoints.Resources.Networks;

public sealed record NetworkView(
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
    bool? ConfigOnly,
    string ConfigFrom = null,
    IPAMView Ipam = null,
    Dictionary<string, string> Options = null,
    Dictionary<string, string> Labels = null
    )
{
    public static NetworksView Map(ListNetworksReply reply)
        => new ([.. reply.Networks.Select(Map)]);

    public static NetworkView Map(NetworkMessage network)
    {
        return new NetworkView(
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
            network.Ipam != null ? new IPAMView(network.Ipam.Driver, IPAMConfigView.Map(network.Ipam.Config?.ToList()), network.Ipam.Options?.ToDictionary()) : null,
            Options: network.Options?.ToDictionary(),
            Labels: network.Labels?.ToDictionary()
        );
    }
};

public sealed record IPAMView(
    string Driver,
    List<IPAMConfigView> Config = null,
    Dictionary<string, string> Options = null
);

public sealed record IPAMConfigView(
    string Subnet,
    string Gateway,
    string IPRange = null
)
{
    internal static List<IPAMConfigView> Map(List<IPAMConfigMessage> ipamConfigs)
    {
        return [.. ipamConfigs.Select(Map)];
    }
    internal static IPAMConfigView Map(IPAMConfigMessage ipamConfig)
    {
        return new IPAMConfigView(
            Subnet: ipamConfig.Subnet,
            Gateway: ipamConfig.Gateway,
            IPRange: ipamConfig.IpRange
        );
    }
}
