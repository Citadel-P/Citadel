using Citadel.Common;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public record NetworkSettingsView(
        string Bridge,
        string SandboxID,
        bool? HairpinMode,
        string LinkLocalIPv6Address,
        int? LinkLocalIPv6PrefixLen,
        IEnumerable<MapFieldPortBindingView> Ports,
        string SandboxKey,
        IEnumerable<Address> SecondaryIPAddresses,
        string EndpointID,
        string Gateway,
        string GlobalIPv6Address,
        int? GlobalIPv6PrefixLen,
        string IPAddress,
        int? IPPrefixLen,
        string IPv6Gateway,
        string MacAddress,
        Dictionary<string, EndpointSettingsView> Networks
    );


public record MapFieldPortBindingView(string Key, IEnumerable<PortBindingView> Value);

public record PortBindingView(string HostIP, string HostPort);
