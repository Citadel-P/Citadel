using Hosting.Common;
using System.Text;

namespace Tests.Integration.Application.Features.Identity.Auth;

public class LoginCommandTests : IntegrationTestBase
{
    [Fact]
    public async Task Handle_ReturnsSuccess_WhenCredentialsValid()
    {
        var userAuth = """
        {
          "email": "admin@citadel.local",
          "password": "admin123"
        }
        """;
        var content = new StringContent(userAuth, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/authentication/login", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.NotNull(responseBody);
    }

    [Fact]
    public async Task Handle_ReturnsFailure_WhenUserDoesNotExist()
    {
        var userAuth = """
        {
          "email": "fake@email.com",
          "password": "invalid@Password"
        }
        """;
        var content = new StringContent(userAuth, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/authentication/login", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Handle_ReturnsFailure_WhenPasswordIsIncorrect()
    {
        var userAuth = """
        {
          "email": "admin@citadel.local",
          "password": "invalid@Password"
        }
        """;
        var content = new StringContent(userAuth, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/authentication/login", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Handle_ReturnsFailure_WhenUserActorIsDisabled()
    {
        await SetActorEnabledAsync(Constants.DefaultAdminId, false);
        Client.DefaultRequestHeaders.Authorization = null;

        var userAuth = """
        {
          "email": "admin@citadel.local",
          "password": "admin123"
        }
        """;
        var content = new StringContent(userAuth, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/authentication/login", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
    }
}
