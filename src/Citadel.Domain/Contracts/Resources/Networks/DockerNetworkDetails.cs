namespace Domain.Contracts.Resources.Networks;

public record DockerNetworkDetails(
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
    IReadOnlyList<NetworkPeerInfo> Peers)
{
    public Guid PlatformId{ get; set; }
    public string? DockerNodeId { get; set; }
    public bool IsSystem => DockerNetworkSystemClassifier.IsSystem(Name, Ingress);
}
