using Citadel.Common;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public record EndpointSettingsView(
        EndpointIPAMConfig IPAMConfig,
        ICollection<string> Links,
        string MacAddress,
        ICollection<string> Aliases,
        string NetworkID,
        string EndpointID,
        string Gateway,
        string IPAddress,
        long? IPPrefixLen,
        string IPv6Gateway,
        string GlobalIPv6Address,
        long? GlobalIPv6PrefixLen,
        IDictionary<string, string> DriverOpts,
        ICollection<string> DNSNames
    );