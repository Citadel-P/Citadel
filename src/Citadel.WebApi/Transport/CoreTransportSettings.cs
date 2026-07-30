using System.Net;
using Hosting.Common;
using Hosting.Common.Security;

namespace WebApi.Transport;

public enum CoreTransportMode
{
    ReverseProxy,
    Direct,
    Disabled
}

internal sealed class CoreTransportSettings : IDisposable
{
    private const int DefaultApiPort = 8000;
    private const int DefaultEdgeGrpcPort = 8001;

    private CoreTransportSettings(
        CoreTransportMode mode,
        Uri publicUrl,
        Uri edgeAgentGrpcUrl,
        int apiPort,
        int edgeGrpcPort,
        IReadOnlyList<IPAddress> knownProxies,
        IReadOnlyList<IPNetwork> knownNetworks,
        int forwardLimit,
        ServerCertificate? certificate)
    {
        Mode = mode;
        PublicUrl = publicUrl;
        EdgeAgentGrpcUrl = edgeAgentGrpcUrl;
        ApiPort = apiPort;
        EdgeGrpcPort = edgeGrpcPort;
        KnownProxies = knownProxies;
        KnownNetworks = knownNetworks;
        ForwardLimit = forwardLimit;
        Certificate = certificate;
    }

    public CoreTransportMode Mode { get; }
    public Uri PublicUrl { get; }
    public Uri EdgeAgentGrpcUrl { get; }
    public int ApiPort { get; }
    public int EdgeGrpcPort { get; }
    public IReadOnlyList<IPAddress> KnownProxies { get; }
    public IReadOnlyList<IPNetwork> KnownNetworks { get; }
    public int ForwardLimit { get; }
    public ServerCertificate? Certificate { get; }
    public bool SecureCookies => Mode is CoreTransportMode.ReverseProxy or CoreTransportMode.Direct;

    public static CoreTransportSettings Load(IConfiguration configuration)
    {
        var mode = ParseMode(configuration["Transport:Mode"]);
        var apiPort = ParsePort(configuration["Transport:ApiPort"], DefaultApiPort, "Transport:ApiPort");
        var edgeGrpcPort = ParsePort(
            configuration["Transport:EdgeGrpcPort"],
            DefaultEdgeGrpcPort,
            "Transport:EdgeGrpcPort");

        if (apiPort == edgeGrpcPort)
        {
            throw new InvalidOperationException(
                "Transport:ApiPort and Transport:EdgeGrpcPort must use different ports.");
        }

        var publicUrl = ParsePublicUrl(
            configuration["Transport:PublicUrl"],
            mode == CoreTransportMode.Disabled ? $"http://localhost:{apiPort}" : null,
            "Transport:PublicUrl",
            mode);
        var edgeAgentGrpcUrl = ParsePublicUrl(
            configuration["EdgeAgent:PublicGrpcUrl"],
            mode == CoreTransportMode.Disabled ? $"http://localhost:{edgeGrpcPort}" : null,
            "EdgeAgent:PublicGrpcUrl",
            mode);

        ValidateAllowedHosts(configuration["AllowedHosts"], mode, publicUrl, edgeAgentGrpcUrl);

        var knownProxies = ParseProxies(configuration["Transport:ForwardedHeaders:KnownProxies"]);
        var knownNetworks = ParseNetworks(configuration["Transport:ForwardedHeaders:KnownNetworks"]);
        var forwardLimit = ParseForwardLimit(configuration["Transport:ForwardedHeaders:ForwardLimit"]);

        var certificateOptionsPresent = HasAnyValue(
            configuration["Transport:Certificate:Path"],
            configuration["Transport:Certificate:PrivateKeyPath"]);
        var proxyOptionsPresent = knownProxies.Count > 0
                                  || knownNetworks.Count > 0
                                  || !string.IsNullOrWhiteSpace(configuration["Transport:ForwardedHeaders:ForwardLimit"]);

        ServerCertificate? certificate = null;
        switch (mode)
        {
            case CoreTransportMode.ReverseProxy:
                if (knownProxies.Count == 0 && knownNetworks.Count == 0)
                {
                    throw new InvalidOperationException(
                        "ReverseProxy mode requires Transport:ForwardedHeaders:KnownProxies or KnownNetworks.");
                }

                if (certificateOptionsPresent)
                {
                    throw new InvalidOperationException(
                        "Transport:Certificate settings are not valid in ReverseProxy mode.");
                }

                break;

            case CoreTransportMode.Direct:
                if (proxyOptionsPresent)
                {
                    throw new InvalidOperationException(
                        "Transport:ForwardedHeaders settings are not valid in Direct mode.");
                }

                certificate = ServerCertificate.Load(
                    configuration["Transport:Certificate:Path"],
                    configuration["Transport:Certificate:PrivateKeyPath"],
                    [publicUrl.Host, edgeAgentGrpcUrl.Host]);
                break;

            case CoreTransportMode.Disabled:
                if (proxyOptionsPresent || certificateOptionsPresent)
                {
                    throw new InvalidOperationException(
                        "Certificate and forwarded-header settings are not valid in Disabled mode.");
                }

                break;

            default:
                throw new InvalidOperationException($"Unsupported transport mode '{mode}'.");
        }

        return new CoreTransportSettings(
            mode,
            publicUrl,
            edgeAgentGrpcUrl,
            apiPort,
            edgeGrpcPort,
            knownProxies,
            knownNetworks,
            forwardLimit,
            certificate);
    }

    public void Dispose() => Certificate?.Dispose();

    private static CoreTransportMode ParseMode(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            if (Helpers.IsDesignTime())
            {
                return CoreTransportMode.Disabled;
            }

            throw new InvalidOperationException(
                "Transport:Mode is required. Use ReverseProxy, Direct, or explicitly opt into Disabled.");
        }

        var candidate = value.Trim();
        return !int.TryParse(candidate, out _)
               && Enum.TryParse<CoreTransportMode>(
                   candidate,
                   ignoreCase: true,
                   out var mode)
               && Enum.IsDefined(mode)
            ? mode
            : throw new InvalidOperationException(
                "Transport:Mode must be ReverseProxy, Direct, or Disabled.");
    }

    private static Uri ParsePublicUrl(
        string? value,
        string? fallback,
        string settingName,
        CoreTransportMode mode)
    {
        var candidate = string.IsNullOrWhiteSpace(value) ? fallback : value.Trim();
        if (!Uri.TryCreate(candidate, UriKind.Absolute, out var uri)
            || string.IsNullOrWhiteSpace(uri.Host)
            || !string.IsNullOrEmpty(uri.UserInfo)
            || !string.IsNullOrEmpty(uri.Query)
            || !string.IsNullOrEmpty(uri.Fragment)
            || uri.AbsolutePath != "/")
        {
            throw new InvalidOperationException(
                $"{settingName} must be an absolute origin without credentials, path, query, or fragment.");
        }

        var expectedScheme = mode == CoreTransportMode.Disabled
            ? Uri.UriSchemeHttp
            : Uri.UriSchemeHttps;
        if (!string.Equals(uri.Scheme, expectedScheme, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException(
                $"{settingName} must use {expectedScheme} when Transport:Mode is {mode}.");
        }

        return new Uri(uri.GetLeftPart(UriPartial.Authority), UriKind.Absolute);
    }

    private static int ParsePort(string? value, int fallback, string settingName)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return fallback;
        }

        return int.TryParse(value, out var port) && port is > 0 and <= 65535
            ? port
            : throw new InvalidOperationException($"{settingName} must be between 1 and 65535.");
    }

    private static int ParseForwardLimit(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return 1;
        }

        return int.TryParse(value, out var limit) && limit > 0
            ? limit
            : throw new InvalidOperationException(
                "Transport:ForwardedHeaders:ForwardLimit must be greater than zero.");
    }

    private static IReadOnlyList<IPAddress> ParseProxies(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return [];
        }

        var proxies = new List<IPAddress>();
        foreach (var candidate in SplitList(value))
        {
            if (!IPAddress.TryParse(candidate, out var address))
            {
                throw new InvalidOperationException(
                    $"Transport:ForwardedHeaders:KnownProxies contains invalid IP address '{candidate}'.");
            }

            if (!proxies.Contains(address))
            {
                proxies.Add(address);
            }
        }

        return proxies;
    }

    private static IReadOnlyList<IPNetwork> ParseNetworks(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return [];
        }

        var networks = new List<IPNetwork>();
        foreach (var candidate in SplitList(value))
        {
            if (!IPNetwork.TryParse(candidate, out var network))
            {
                throw new InvalidOperationException(
                    $"Transport:ForwardedHeaders:KnownNetworks contains invalid CIDR '{candidate}'.");
            }

            if (!networks.Contains(network))
            {
                networks.Add(network);
            }
        }

        return networks;
    }

    private static IEnumerable<string> SplitList(string value)
    {
        foreach (var candidate in value.Split(',', StringSplitOptions.TrimEntries))
        {
            if (candidate.Length == 0)
            {
                throw new InvalidOperationException(
                    "Transport forwarded-header lists cannot contain empty entries.");
            }

            yield return candidate;
        }
    }

    private static void ValidateAllowedHosts(
        string? value,
        CoreTransportMode mode,
        Uri publicUrl,
        Uri edgeAgentGrpcUrl)
    {
        if (mode == CoreTransportMode.Disabled)
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(value) || value.Trim() == "*")
        {
            throw new InvalidOperationException(
                "AllowedHosts must explicitly include the configured production public hosts.");
        }

        var hosts = value
            .Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        if (!hosts.Contains(publicUrl.Host) || !hosts.Contains(edgeAgentGrpcUrl.Host))
        {
            throw new InvalidOperationException(
                "AllowedHosts must include the hosts from Transport:PublicUrl and EdgeAgent:PublicGrpcUrl.");
        }
    }

    private static bool HasAnyValue(params string?[] values)
        => values.Any(value => !string.IsNullOrWhiteSpace(value));
}
