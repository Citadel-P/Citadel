using Citadel.Common;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public record NetworkSettingsView(
        string Bridge,
        string SandboxID,
        bool? HairpinMode,
        string LinkLocalIPv6Address,
        long? LinkLocalIPv6PrefixLen,
        IEnumerable<MapFieldPortBindingView> Ports,
        string SandboxKey,
        IEnumerable<Address> SecondaryIPAddresses,
        string EndpointID,
        string Gateway,
        string GlobalIPv6Address,
        long? GlobalIPv6PrefixLen,
        string IPAddress,
        long? IPPrefixLen,
        string IPv6Gateway,
        string MacAddress,
        Dictionary<string, EndpointSettingsView> Networks
    );


public record MapFieldPortBindingView(string Key, IEnumerable<PortBindingView> Value);

public record PortBindingView(string HostIP, string HostPort);

internal static class MapperExtension
{
    public static NetworkSettingsView Map(this NetworkSettings networkSettings) => new (
          Bridge: networkSettings.Bridge,
            SandboxID: networkSettings.SandboxID,
            HairpinMode: networkSettings.HairpinMode,
            LinkLocalIPv6Address: networkSettings.LinkLocalIPv6Address,
            LinkLocalIPv6PrefixLen: networkSettings.LinkLocalIPv6PrefixLen,
            Ports: networkSettings.Ports.Select(x => new MapFieldPortBindingView(x.Key, x.Value?.Select(Map) ?? [])),
            SandboxKey: networkSettings.SandboxKey,
            SecondaryIPAddresses: networkSettings.SecondaryIPAddresses,
            EndpointID: networkSettings.EndpointID,
            Gateway: networkSettings.Gateway,
            GlobalIPv6Address: networkSettings.GlobalIPv6Address,
            GlobalIPv6PrefixLen: networkSettings.GlobalIPv6PrefixLen,
            IPAddress: networkSettings.IpAddress,
            IPPrefixLen: networkSettings.IpPrefixLen,
            IPv6Gateway: networkSettings.Ipv6Gateway,
            MacAddress: networkSettings.MacAddress,
            Networks: networkSettings.Networks.ToDictionary(
                x => x.Key,
                x => new EndpointSettingsView(
                    IPAMConfig: x.Value.IPAMConfig,
                    Links: x.Value.Links,
                    MacAddress: x.Value.MacAddress,
                    Aliases: x.Value.Aliases,
                    NetworkID: x.Value.NetworkID,
                    EndpointID: x.Value.EndpointID,
                    Gateway: x.Value.Gateway,
                    IPAddress: x.Value.IpAddress,
                    IPPrefixLen: x.Value.IpPrefixLen,
                    IPv6Gateway: x.Value.Ipv6Gateway,
                    GlobalIPv6Address: x.Value.GlobalIPv6Address,
                    GlobalIPv6PrefixLen: x.Value.GlobalIPv6PrefixLen,
                    DriverOpts: x.Value.DriverOpts.ToDictionary(),
                    DNSNames: x.Value.DNSNames
                )
            )
    );

    internal static PortBindingView Map(this PortBinding portBinding) => new(
        HostIP: portBinding.HostPort,
        HostPort: portBinding.HostPort
        );
}
