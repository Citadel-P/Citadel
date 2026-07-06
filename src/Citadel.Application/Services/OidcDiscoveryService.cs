using System.Text.Json;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services;

public interface IOidcDiscoveryService
{
    Task<Result<OidcDiscoveryResult>> GetDiscoveryAsync(string issuer, CancellationToken cancellationToken);
    Task<Result<OidcDiscoveryResult>> TestDiscoveryAsync(string issuer, CancellationToken cancellationToken);
}

public sealed record OidcDiscoveryResult(
    string Issuer,
    string AuthorizationEndpoint,
    string TokenEndpoint,
    string JwksUri);

internal sealed class OidcDiscoveryService : IOidcDiscoveryService
{
    private static readonly HttpClient HttpClient = new()
    {
        Timeout = TimeSpan.FromSeconds(10)
    };

    public async Task<Result<OidcDiscoveryResult>> TestDiscoveryAsync(string issuer, CancellationToken cancellationToken)
        => await GetDiscoveryAsync(issuer, cancellationToken);

    public async Task<Result<OidcDiscoveryResult>> GetDiscoveryAsync(string issuer, CancellationToken cancellationToken)
    {
        var normalizedIssuer = issuer.Trim().TrimEnd('/');
        if (!Uri.TryCreate(normalizedIssuer, UriKind.Absolute, out var issuerUri)
            || issuerUri.Scheme is not "https" and not "http")
        {
            return Result.Failure<OidcDiscoveryResult>(new BadRequestError("Issuer must be an absolute HTTP or HTTPS URL."));
        }

        var discoveryUri = new Uri($"{normalizedIssuer}/.well-known/openid-configuration");
        try
        {
            using var response = await HttpClient.GetAsync(discoveryUri, cancellationToken);
            if (!response.IsSuccessStatusCode)
            {
                return Result.Failure<OidcDiscoveryResult>(
                    new BadRequestError($"OIDC discovery failed with status code {(int)response.StatusCode}."));
            }

            await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
            using var document = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);
            var root = document.RootElement;

            var metadataIssuer = GetRequiredString(root, "issuer");
            var authorizationEndpoint = GetRequiredString(root, "authorization_endpoint");
            var tokenEndpoint = GetRequiredString(root, "token_endpoint");
            var jwksUri = GetRequiredString(root, "jwks_uri");

            if (!string.Equals(metadataIssuer.TrimEnd('/'), normalizedIssuer, StringComparison.Ordinal))
            {
                return Result.Failure<OidcDiscoveryResult>(
                    new BadRequestError("Discovery issuer does not match the configured issuer."));
            }

            if (!IsAbsoluteUri(authorizationEndpoint) || !IsAbsoluteUri(tokenEndpoint) || !IsAbsoluteUri(jwksUri))
            {
                return Result.Failure<OidcDiscoveryResult>(
                    new BadRequestError("OIDC discovery endpoints must be absolute URLs."));
            }

            return Result.Success(new OidcDiscoveryResult(
                metadataIssuer,
                authorizationEndpoint,
                tokenEndpoint,
                jwksUri));
        }
        catch (JsonException)
        {
            return Result.Failure<OidcDiscoveryResult>(new BadRequestError("OIDC discovery response is not valid JSON."));
        }
        catch (HttpRequestException ex)
        {
            return Result.Failure<OidcDiscoveryResult>(new BadRequestError($"OIDC discovery request failed: {ex.Message}"));
        }
        catch (TaskCanceledException)
        {
            return Result.Failure<OidcDiscoveryResult>(new BadRequestError("OIDC discovery request timed out."));
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure<OidcDiscoveryResult>(new BadRequestError(ex.Message));
        }
    }

    private static string GetRequiredString(JsonElement root, string propertyName)
    {
        if (!root.TryGetProperty(propertyName, out var property)
            || property.ValueKind != JsonValueKind.String
            || string.IsNullOrWhiteSpace(property.GetString()))
        {
            throw new InvalidOperationException($"OIDC discovery response is missing {propertyName}.");
        }

        return property.GetString()!;
    }

    private static bool IsAbsoluteUri(string value)
        => Uri.TryCreate(value, UriKind.Absolute, out var uri)
           && uri.Scheme is "https" or "http";
}
