using System.Net.Http.Json;
using System.Text.Json;
using System.Threading;
using Application.Features.Images.Queries;
using Domain.Contracts.Resources.Registries;
using Microsoft.AspNetCore.Http.HttpResults;

namespace Tests.Integration.Application.Features.Images;

public class GetDockerHubPublicImagesTests : IntegrationTestBase<WebApi.Program>
{
    [Fact]
    public async Task GetDockerHubPublicImages_ReturnsDefaultImages_WhenNoImageNameProvided()
    {
        // Arrange
        var request = new HttpRequestMessage(HttpMethod.Get, "/api/v1/images/dockerhub");

        // Act
        var response = await Client.SendAsync(request, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var images = (await response.Content.ReadFromJsonAsync(typeof(IEnumerable<DockerHubImageResult>), cancellationToken: TestContext.Current.CancellationToken)) 
                        as IEnumerable<DockerHubImageResult>;

        Assert.Contains(images ?? [], s => s.Name == "nginx");
    }
}
