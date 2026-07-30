using Domain.Configs;

namespace Application.Configs;

internal static class AgentTransportAddressPolicy
{
    public static string? GetValidationError(
        string? address,
        AgentTransportOptions options)
    {
        if (!Uri.TryCreate(address, UriKind.Absolute, out var uri)
            || string.IsNullOrWhiteSpace(uri.Host)
            || (uri.Scheme != Uri.UriSchemeHttp
                && uri.Scheme != Uri.UriSchemeHttps)
            || !string.IsNullOrEmpty(uri.UserInfo)
            || !string.IsNullOrEmpty(uri.Query)
            || !string.IsNullOrEmpty(uri.Fragment)
            || uri.AbsolutePath != "/")
        {
            return "Agent endpoint must be an absolute HTTP or HTTPS origin without credentials, path, query, or fragment.";
        }

        if (!options.AllowInsecure && uri.Scheme != Uri.UriSchemeHttps)
        {
            return "Agent endpoint must use HTTPS because insecure Agent transport is disabled.";
        }

        return null;
    }
}
