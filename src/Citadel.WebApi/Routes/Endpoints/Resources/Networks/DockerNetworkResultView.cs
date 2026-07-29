using Domain.Contracts.Resources.Networks;
using WebApi.Routes.Endpoints.Resources.Identity;

public sealed record DockerNetworkResultView(
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
    IReadOnlyDictionary<string, string> Labels,
    bool IsSystem,
    NetworkCapabilities? Capabilities = null)
{
    internal static DockerNetworkResultView Map(DockerNetworkResult dockerNetworkResult)
    {
        return new DockerNetworkResultView(
            dockerNetworkResult.Name,
            dockerNetworkResult.Id,
            dockerNetworkResult.Created,
            dockerNetworkResult.Driver,
            dockerNetworkResult.Scope,
            dockerNetworkResult.EnableIPv4,
            dockerNetworkResult.EnableIPv6,
            dockerNetworkResult.Internal,
            dockerNetworkResult.Attachable,
            dockerNetworkResult.Ingress,
            dockerNetworkResult.ConfigOnly,
            dockerNetworkResult.InUse,
            dockerNetworkResult.ConfigFrom,
            dockerNetworkResult.Ipam,
            dockerNetworkResult.Options,
            dockerNetworkResult.Labels,
            dockerNetworkResult.IsSystem);
    }
}
