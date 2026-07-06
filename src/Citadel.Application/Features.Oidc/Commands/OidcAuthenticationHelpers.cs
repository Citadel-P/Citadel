using System.Security.Cryptography;
using System.Text;

namespace Application.Features.Oidc.Commands;

internal static class OidcOpaqueValue
{
    public static string CreateOpaqueValue(int bytes)
        => Convert.ToBase64String(RandomNumberGenerator.GetBytes(bytes))
            .TrimEnd('=')
            .Replace('+', '-')
            .Replace('/', '_');

    public static string HashOpaqueValue(string value)
        => Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(value))).ToLowerInvariant();
}

internal static class OidcLoginUrl
{
    public static string? NormalizeReturnUrl(
        string value,
        string redirectUri,
        IReadOnlyCollection<string>? allowedReturnOrigins = null)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Any(char.IsControl))
            return null;

        if (Uri.TryCreate(value, UriKind.Relative, out var relative)
            && value.StartsWith("/", StringComparison.Ordinal)
            && !value.StartsWith("//", StringComparison.Ordinal)
            && !value.StartsWith("/\\", StringComparison.Ordinal))
        {
            return relative.ToString();
        }

        if (!Uri.TryCreate(value, UriKind.Absolute, out var absolute) || absolute.Scheme is not ("http" or "https"))
            return null;

        if (!Uri.TryCreate(redirectUri, UriKind.Absolute, out var callbackUri))
            return null;

        if (UriOriginsEqual(absolute, callbackUri) || IsAllowedOrigin(absolute, allowedReturnOrigins))
            return absolute.ToString();

        return null;
    }

    private static bool IsAllowedOrigin(Uri uri, IReadOnlyCollection<string>? allowedOrigins)
        => allowedOrigins is not null
           && allowedOrigins
               .Select(origin => Uri.TryCreate(origin, UriKind.Absolute, out var allowed) ? allowed : null)
               .Where(allowed => allowed is not null)
               .Any(allowed => UriOriginsEqual(uri, allowed!));

    private static bool UriOriginsEqual(Uri left, Uri right)
        => string.Equals(left.Scheme, right.Scheme, StringComparison.OrdinalIgnoreCase)
           && string.Equals(left.IdnHost, right.IdnHost, StringComparison.OrdinalIgnoreCase)
           && left.Port == right.Port;
}
