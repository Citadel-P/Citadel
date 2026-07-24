using System.Text;
using Hosting.Common;

namespace Tests.Integration.Application.Features.Identity.Auth;

public class RefreshTokenCommandTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Handle_ReturnsSuccess_WhenCredentialsValid()
    {
        // Arrange
        var refreshToken = await GetRefreshToken();
        Client.DefaultRequestHeaders.Add("Cookie", $"{Constants.RefreshToken}={refreshToken}");

        // Act
        var response = await Client.GetAsync("/api/v1/authentication/refresh", cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.NotNull(responseBody);
    }

    [Fact]
    public async Task Handle_UsesCanonicalCookie_WhenLegacyCookieIsAlsoPresent()
    {
        var refreshToken = await GetRefreshToken();
        Client.DefaultRequestHeaders.Add(
            "Cookie",
            $"{Constants.RefreshToken}={refreshToken}; {Constants.RefreshToken}=stale_legacy_token");

        var response = await Client.GetAsync(
            "/api/v1/authentication/refresh",
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var setCookieHeaders = GetSetCookieHeaders(response);
        Assert.Contains(setCookieHeaders, header => IsCookieForPath(header, "/", hasValue: false));
        Assert.Contains(setCookieHeaders, header => IsCookieForPath(header, "/api/v1/authentication", hasValue: true));
    }

    [Fact]
    public async Task Handle_DoesNotFallBackToLegacyCookie_WhenCanonicalCookieIsInvalid()
    {
        var refreshToken = await GetRefreshToken();
        Client.DefaultRequestHeaders.Add(
            "Cookie",
            $"{Constants.RefreshToken}=invalid_canonical_token; {Constants.RefreshToken}={refreshToken}");

        var response = await Client.GetAsync(
            "/api/v1/authentication/refresh",
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Unauthorized, response.StatusCode);
    }

    [Fact]
    public async Task Handle_ReturnsFailure_WhenRefreshTokenIsMissing()
    {
        // Act
        var response = await Client.GetAsync("/api/v1/authentication/refresh", cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.False(response.IsSuccessStatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Handle_ReturnsFailure_WhenUserActorIsDisabled()
    {
        var refreshToken = await GetRefreshToken();
        await SetActorEnabledAsync(Constants.DefaultAdminId, false);

        Client.DefaultRequestHeaders.Authorization = null;
        Client.DefaultRequestHeaders.Remove("Cookie");
        Client.DefaultRequestHeaders.Add("Cookie", $"{Constants.RefreshToken}={refreshToken}");

        var response = await Client.GetAsync("/api/v1/authentication/refresh", cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Unauthorized, response.StatusCode);
    }

    [Fact]
    public async Task Handle_ReturnsFailure_WhenRefreshTokenIsInvalid()
    {
        // Arrange
        Client.DefaultRequestHeaders.Add("Cookie", $"{Constants.RefreshToken}=invalid_token_value");

        // Act
        var response = await Client.GetAsync("/api/v1/authentication/refresh", cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.False(response.IsSuccessStatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    private async Task<string> GetRefreshToken()
    {
        var userAuth = """
        {
          "emailOrName": "admin@citadel.local",
          "password": "admin123"
        }
        """;
        var content = new StringContent(userAuth, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/authentication/login", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        foreach (var header in GetSetCookieHeaders(response))
        {
            var cookie = header.Split(';', 2)[0].Trim();
            var prefix = $"{Constants.RefreshToken}=";
            if (cookie.StartsWith(prefix, StringComparison.Ordinal) && cookie.Length > prefix.Length)
                return cookie[prefix.Length..];
        }

        throw new InvalidOperationException("Refresh token not found in response cookies.");
    }

    private static string[] GetSetCookieHeaders(HttpResponseMessage response)
        => response.Headers.TryGetValues("Set-Cookie", out var values)
            ? values.ToArray()
            : [];

    private static bool IsCookieForPath(string header, string path, bool hasValue)
    {
        var parts = header.Split(';', StringSplitOptions.TrimEntries);
        var cookiePrefix = $"{Constants.RefreshToken}=";
        if (!parts[0].StartsWith(cookiePrefix, StringComparison.Ordinal))
            return false;

        var cookieHasValue = parts[0].Length > cookiePrefix.Length;
        var expectedPath = $"Path={path}";
        return cookieHasValue == hasValue
               && parts.Any(part => string.Equals(part, expectedPath, StringComparison.OrdinalIgnoreCase));
    }
}
