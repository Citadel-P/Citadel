namespace Domain.Contracts.Resources.Networks;

public record DockerNetworkResult(
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
    bool InUse,
    string? ConfigFrom,
    IpAddressManagementConfig? Ipam,
    IReadOnlyDictionary<string, string> Options,
    IReadOnlyDictionary<string, string> Labels)
{
    public Guid PlatformId { get; set; }
    public bool IsSystem => DockerNetworkSystemClassifier.IsSystem(Name, Ingress);
}

public record IpamSubnetConfiguration(
    string? Subnet,
    string? IpRange,
    string? Gateway
);

public record IpAddressManagementConfig(
    string? Driver,
    IReadOnlyList<IpamSubnetConfiguration> Config,
    IReadOnlyDictionary<string, string> Options);

public record NetworkConnectedContainer(
    string Name,
    string EndpointId,
    string MacAddress,
    string IpV4Address,
    string Ipv6Address
);

public record NetworkPeerInfo(string Name, string Ip);
