using System.Text;

namespace Tests.Integration.Application.Features.Auth;

public class LoginCommandTests : IntegrationTestBase<WebApi.Program>
{
    [Fact]
    public async Task Handle_ReturnsSuccess_WhenCredentialsValid()
    {
        var userAuth = """
        {
          "email": "admin@admin.com",
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
          "email": "admin@admin.com",
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
}
