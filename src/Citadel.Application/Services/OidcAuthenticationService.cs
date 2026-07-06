using System.Security.Claims;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Domain.Entities.Oidc;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.IdentityModel.JsonWebTokens;
using Microsoft.IdentityModel.Tokens;

namespace Application.Services;

public interface IOidcAuthenticationService
{
    OidcAuthorizationRequest CreateAuthorizationRequest(
        OidcProvider provider,
        OidcDiscoveryResult discovery,
        string redirectUri,
        string state,
        string nonce,
        string codeVerifier);

    Task<Result<OidcTokenIdentity>> ExchangeAndValidateAsync(
        OidcProvider provider,
        OidcDiscoveryResult discovery,
        string clientSecret,
        string code,
        string redirectUri,
        string codeVerifier,
        string nonce,
        CancellationToken cancellationToken);
}

public sealed record OidcAuthorizationRequest(string Url, string CodeVerifier);

public sealed record OidcTokenIdentity(
    string Subject,
    string? Email,
    bool EmailVerified,
    string? Name,
    IReadOnlyCollection<Claim> Claims);

internal sealed class OidcAuthenticationService : IOidcAuthenticationService
{
    private static readonly HttpClient HttpClient = new()
    {
        Timeout = TimeSpan.FromSeconds(15)
    };

    public OidcAuthorizationRequest CreateAuthorizationRequest(
        OidcProvider provider,
        OidcDiscoveryResult discovery,
        string redirectUri,
        string state,
        string nonce,
        string codeVerifier)
    {
        var codeChallenge = Base64UrlEncode(SHA256.HashData(Encoding.ASCII.GetBytes(codeVerifier)));
        var query = new Dictionary<string, string>
        {
            ["response_type"] = "code",
            ["client_id"] = provider.ClientId,
            ["redirect_uri"] = redirectUri,
            ["scope"] = provider.Scopes,
            ["state"] = state,
            ["nonce"] = nonce,
            ["code_challenge"] = codeChallenge,
            ["code_challenge_method"] = "S256"
        };

        return new OidcAuthorizationRequest(
            $"{discovery.AuthorizationEndpoint}?{BuildQueryString(query)}",
            codeVerifier);
    }

    public async Task<Result<OidcTokenIdentity>> ExchangeAndValidateAsync(
        OidcProvider provider,
        OidcDiscoveryResult discovery,
        string clientSecret,
        string code,
        string redirectUri,
        string codeVerifier,
        string nonce,
        CancellationToken cancellationToken)
    {
        var tokenResult = await ExchangeCodeAsync(
            discovery.TokenEndpoint,
            provider.ClientId,
            clientSecret,
            code,
            redirectUri,
            codeVerifier,
            cancellationToken);

        if (!tokenResult.IsSuccess(out var idToken))
            return Result.Failure<OidcTokenIdentity>(tokenResult.Errors);

        return await ValidateIdTokenAsync(provider, discovery, idToken, nonce, cancellationToken);
    }

    private static async Task<Result<string>> ExchangeCodeAsync(
        string tokenEndpoint,
        string clientId,
        string clientSecret,
        string code,
        string redirectUri,
        string codeVerifier,
        CancellationToken cancellationToken)
    {
        var fields = new Dictionary<string, string>
        {
            ["grant_type"] = "authorization_code",
            ["code"] = code,
            ["redirect_uri"] = redirectUri,
            ["client_id"] = clientId,
            ["code_verifier"] = codeVerifier
        };

        if (!string.IsNullOrWhiteSpace(clientSecret))
            fields["client_secret"] = clientSecret;

        try
        {
            using var response = await HttpClient.PostAsync(tokenEndpoint, new FormUrlEncodedContent(fields), cancellationToken);
            await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
            using var document = await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken);

            if (!response.IsSuccessStatusCode)
            {
                var error = TryGetString(document.RootElement, "error_description")
                    ?? TryGetString(document.RootElement, "error")
                    ?? $"OIDC token endpoint failed with status code {(int)response.StatusCode}.";

                return Result.Failure<string>(new BadRequestError(error));
            }

            var idToken = TryGetString(document.RootElement, "id_token");
            if (string.IsNullOrWhiteSpace(idToken))
                return Result.Failure<string>(new BadRequestError("OIDC token response did not include an ID token."));

            return Result.Success(idToken);
        }
        catch (JsonException)
        {
            return Result.Failure<string>(new BadRequestError("OIDC token response is not valid JSON."));
        }
        catch (HttpRequestException ex)
        {
            return Result.Failure<string>(new BadRequestError($"OIDC token request failed: {ex.Message}"));
        }
        catch (TaskCanceledException)
        {
            return Result.Failure<string>(new BadRequestError("OIDC token request timed out."));
        }
    }

    private static async Task<Result<OidcTokenIdentity>> ValidateIdTokenAsync(
        OidcProvider provider,
        OidcDiscoveryResult discovery,
        string idToken,
        string nonce,
        CancellationToken cancellationToken)
    {
        try
        {
            var jwksJson = await HttpClient.GetStringAsync(discovery.JwksUri, cancellationToken);
            var signingKeys = new JsonWebKeySet(jwksJson).GetSigningKeys();
            var validationParameters = new TokenValidationParameters
            {
                ValidIssuer = discovery.Issuer,
                ValidAudience = provider.ClientId,
                ValidateIssuer = true,
                ValidateAudience = true,
                ValidateLifetime = true,
                ValidateIssuerSigningKey = true,
                IssuerSigningKeys = signingKeys,
                ClockSkew = TimeSpan.FromMinutes(2)
            };

            var handler = new JsonWebTokenHandler();
            var validationResult = await handler.ValidateTokenAsync(idToken, validationParameters);
            if (!validationResult.IsValid)
            {
                return Result.Failure<OidcTokenIdentity>(
                    new BadRequestError($"OIDC ID token validation failed: {validationResult.Exception?.Message ?? "invalid token"}"));
            }

            var token = handler.ReadJsonWebToken(idToken);
            var claims = token.Claims.ToArray();
            var tokenNonce = claims.FirstOrDefault(static claim => claim.Type == "nonce")?.Value;
            if (!string.Equals(tokenNonce, nonce, StringComparison.Ordinal))
                return Result.Failure<OidcTokenIdentity>(new BadRequestError("OIDC ID token nonce is invalid."));

            var subject = claims.FirstOrDefault(static claim => claim.Type == "sub")?.Value;
            if (string.IsNullOrWhiteSpace(subject))
                return Result.Failure<OidcTokenIdentity>(new BadRequestError("OIDC ID token subject is missing."));

            var email = claims.FirstOrDefault(static claim => claim.Type == "email")?.Value;
            var emailVerified = bool.TryParse(claims.FirstOrDefault(static claim => claim.Type == "email_verified")?.Value, out var verified)
                && verified;
            var name = claims.FirstOrDefault(static claim => claim.Type == "preferred_username")?.Value
                ?? claims.FirstOrDefault(static claim => claim.Type == "name")?.Value
                ?? email;

            return Result.Success(new OidcTokenIdentity(subject, email, emailVerified, name, claims));
        }
        catch (JsonException)
        {
            return Result.Failure<OidcTokenIdentity>(new BadRequestError("OIDC JWKS response is not valid JSON."));
        }
        catch (HttpRequestException ex)
        {
            return Result.Failure<OidcTokenIdentity>(new BadRequestError($"OIDC JWKS request failed: {ex.Message}"));
        }
        catch (TaskCanceledException)
        {
            return Result.Failure<OidcTokenIdentity>(new BadRequestError("OIDC JWKS request timed out."));
        }
    }

    private static string? TryGetString(JsonElement root, string propertyName)
        => root.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.String
            ? property.GetString()
            : null;

    private static string BuildQueryString(IEnumerable<KeyValuePair<string, string>> query)
        => string.Join('&', query.Select(static item => $"{Uri.EscapeDataString(item.Key)}={Uri.EscapeDataString(item.Value)}"));

    private static string Base64UrlEncode(byte[] bytes)
        => Convert.ToBase64String(bytes)
            .TrimEnd('=')
            .Replace('+', '-')
            .Replace('/', '_');
}
