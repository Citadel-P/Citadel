namespace Domain.Contracts.Resources.Networks;

public sealed record CreateNetworkCommand(
    string PlatformAddress,
    string Name,
    string? Driver,
    string? Scope,
    bool? Internal,
    bool? Attachable,
    bool? Ingress,
    bool? ConfigOnly,
    NetworkConfigFrom? ConfigFrom,
    IpAddressManagementConfig? Ipam,
    bool? EnableIPv6,
    bool? EnableIPv4,
    IReadOnlyDictionary<string, string> Options,
    IReadOnlyDictionary<string, string> Labels);

public record NetworkConfigFrom(string Network);
