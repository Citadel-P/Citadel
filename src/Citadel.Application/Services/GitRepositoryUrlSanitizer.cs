namespace Application.Services;

internal static class GitRepositoryUrlSanitizer
{
    public static string? Sanitize(string? url)
    {
        if (string.IsNullOrWhiteSpace(url))
            return url;

        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri) || string.IsNullOrWhiteSpace(uri.UserInfo))
            return url;

        var builder = new UriBuilder(uri)
        {
            UserName = string.Empty,
            Password = string.Empty
        };

        return builder.Uri.ToString().TrimEnd('/');
    }
}
