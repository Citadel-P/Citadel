using Application.Services;
using Microsoft.AspNetCore.Http.Metadata;
using System.Text;
using System.Text.Json;

namespace WebApi.Routes;

internal sealed class EndpointDataSourceAutomationApiEndpointCatalog(EndpointDataSource endpointDataSource) : IAutomationApiEndpointCatalog
{
    private readonly Lazy<string> json = new(() => BuildJson(endpointDataSource));

    public string Json => json.Value;

    private static string BuildJson(EndpointDataSource endpointDataSource)
    {
        var endpoints = endpointDataSource.Endpoints
            .OfType<RouteEndpoint>()
            .Select(ReadEndpoint)
            .Where(endpoint => endpoint is not null)
            .Cast<AutomationApiEndpoint>()
            .Where(IsAllowed)
            .ToArray();

        var builder = new StringBuilder();
        builder.Append('[');

        for (var i = 0; i < endpoints.Length; i++)
        {
            if (i > 0)
                builder.Append(',');

            var endpoint = endpoints[i];
            builder.Append('{');
            AppendJsonProperty(builder, "key", endpoint.Key);
            builder.Append(',');
            AppendJsonProperty(builder, "method", endpoint.Method);
            builder.Append(',');
            AppendJsonProperty(builder, "path", endpoint.Path);
            builder.Append(',');
            AppendJsonProperty(builder, "group", endpoint.Group);
            builder.Append('}');
        }

        builder.Append(']');
        return builder.ToString();
    }

    private static AutomationApiEndpoint? ReadEndpoint(RouteEndpoint endpoint)
    {
        var key = endpoint.Metadata.GetMetadata<IEndpointNameMetadata>()?.EndpointName;
        var method = endpoint.Metadata.GetMetadata<IHttpMethodMetadata>()?.HttpMethods.FirstOrDefault();
        var tag = endpoint.Metadata.GetMetadata<ITagsMetadata>()?.Tags.FirstOrDefault();
        var path = NormalizePath(endpoint.RoutePattern.RawText);

        if (string.IsNullOrWhiteSpace(key)
            || string.IsNullOrWhiteSpace(method)
            || string.IsNullOrWhiteSpace(tag)
            || string.IsNullOrWhiteSpace(path))
        {
            return null;
        }

        return new AutomationApiEndpoint(key, method.ToUpperInvariant(), path, tag, ToGroupName(tag));
    }

    private static string? NormalizePath(string? path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return null;

        var normalized = path.StartsWith('/') ? path : $"/{path}";
        return normalized.Length > 1 ? normalized.TrimEnd('/') : normalized;
    }

    private static bool IsAllowed(AutomationApiEndpoint endpoint)
    {
        if (endpoint.Tag is "Authentication" or "AutomationActions" or "WebhookListener")
            return false;

        var lower = endpoint.Path.ToLowerInvariant();
        return lower.StartsWith("/api/v1/")
            && !lower.StartsWith("/api/v1/authentication")
            && !lower.StartsWith("/api/v1/automation")
            && !lower.StartsWith("/api/v1/resourcebindings/secrets")
            && !lower.StartsWith("/api/v1/resourcebindings/secret-providers")
            && !lower.Contains("/terminal")
            && !lower.Contains("/exec");
    }

    private static string ToGroupName(string tag)
        => string.IsNullOrEmpty(tag) ? "default" : char.ToLowerInvariant(tag[0]) + tag[1..];

    private static void AppendJsonProperty(StringBuilder builder, string name, string value)
    {
        builder.Append(JsonStringLiteral(name));
        builder.Append(':');
        builder.Append(JsonStringLiteral(value));
    }

    private static string JsonStringLiteral(string value)
        => $"\"{JsonEncodedText.Encode(value).ToString()}\"";

    private sealed record AutomationApiEndpoint(string Key, string Method, string Path, string Tag, string Group);
}
