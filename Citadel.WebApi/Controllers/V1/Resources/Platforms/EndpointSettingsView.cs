using System.Text.Json.Serialization;

namespace WebApi.Controllers.V1.Resources.Platforms;

public sealed record EndpointSettingsView
{
    [JsonPropertyName("links")]
    public ICollection<string> Links { get; set; }

    [JsonPropertyName("macAddress")]
    public string MacAddress { get; set; }

    [JsonPropertyName("aliases")]
    public ICollection<string> Aliases { get; set; }

    [JsonPropertyName("networkID")]
    public string NetworkID { get; set; }

    [JsonPropertyName("endpointID")]
    public string EndpointID { get; set; }

    [JsonPropertyName("gateway")]
    public string Gateway { get; set; }

    [JsonPropertyName("iPAddress")]
    public string IPAddress { get; set; }

    [JsonPropertyName("iPPrefixLen")]
    public int? IPPrefixLen { get; set; }

    [JsonPropertyName("iPv6Gateway")]
    public string IPv6Gateway { get; set; }

    [JsonPropertyName("globalIPv6Address")]
    public string GlobalIPv6Address { get; set; }

    [JsonPropertyName("globalIPv6PrefixLen")]
    public long? GlobalIPv6PrefixLen { get; set; }

    //[JsonPropertyName("DriverOpts")]
    //public IDictionary<string, string> DriverOpts { get; set; }

    [JsonPropertyName("dnsNames")]
    public ICollection<string> DNSNames { get; set; }
}