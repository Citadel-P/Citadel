using System.Text;
using Hosting.Common;

namespace Tests.Integration.Application.Features.Auth;

public class RefreshTokenCommandTests : IntegrationTestBase
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
          "email": "admin@admin.com",
          "password": "admin123"
        }
        """;
        var content = new StringContent(userAuth, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/authentication/login", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        if (response.Headers.TryGetValues("Set-Cookie", out var setCookieHeaders))
        {
            foreach (var header in setCookieHeaders)
            {
                // Looking for the refresh_token cookie
                var cookies = header.Split(';');
                foreach (var cookie in cookies)
                {
                    var trimmed = cookie.Trim();
                    if (trimmed.StartsWith($"{Constants.RefreshToken}="))
                    {
                        return trimmed.Substring($"{Constants.RefreshToken}=".Length);
                    }
                }
            }
        }

        throw new InvalidOperationException("Refresh token not found in response cookies."); ;
    }
}
