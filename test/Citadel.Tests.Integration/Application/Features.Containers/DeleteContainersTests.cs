using System.Text;

using System.Net;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Tests.Integration.Helpers;
using Microsoft.Extensions.DependencyInjection;

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

    [Fact]
    public async Task Delete_StaleContainerOnlyInDatabase_ReturnsSuccessAndDeletesRow()
    {
        var platform = Fakes.GetDummyPlatform();
        const string containerId = "42ccd07956a642ccd07956a642ccd07956a642ccd07956a642ccd07956a6";
        var container = new Container(
            name: "stale-container",
            dockerImageId: "sha256:stale",
            platformId: platform.Id,
            dockerContainerId: containerId,
            state: ContainerStateStatus.Offline);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var content = $$"""
        {
            "containerIds": [
                "{{containerId[..12]}}"
            ],
            "v": false,
            "force": false,
            "link": false
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/containers")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deleted = await assertUow.Containers.GetByIdAsync(containerId, TestContext.Current.CancellationToken);
        Assert.Null(deleted);
    }
}
