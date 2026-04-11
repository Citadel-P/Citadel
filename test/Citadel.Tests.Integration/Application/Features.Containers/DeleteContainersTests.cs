using System.Text;

namespace Tests.Integration.Application.Features.Containers;

public class DeleteContainersTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Delete_Container_WithInvalidParams_ReturnsBadRequest()
    {
        // Arrange
        var content = """
        {
            "containerIds": [
                ""
            ],
            "v": false,
            "force": false,
            "link": false
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/containers")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
