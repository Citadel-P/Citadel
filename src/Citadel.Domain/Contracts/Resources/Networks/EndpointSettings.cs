namespace Domain.Contracts.Resources.Networks;

public record EndpointSettings(
    EndpointIPAMConfig? IPAMConfig = null, 
    string? MacAddress = null, 
    string? NetworkID = null,
    string? EndpointID = null,
    string? Gateway = null,
    string? IPAddress = null,
    int? IPPrefixLen = null,
    string? IPv6Gateway = null,
    string? GlobalIPv6Address = null,
    long? GlobalIPv6PrefixLen = null,
    IList<string>? Links = null,
    IList<string>? DNSNames = null,
    IDictionary<string, string>? DriverOpts = null
    );

public record EndpointIPAMConfig(
    string? IPv4Address = null,
    string? IPv6Address = null,
    IList<string>? LinkLocalIPs = null
);