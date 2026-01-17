using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Containers;

public class GetContainerTests : IntegrationTestBase
{
    readonly string containerId = "42ccd07956a6";
    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);

        var image = new Image("image-01", ["sha256:abcd1234"], "image0123", 1000, 1, platform.Id, new DateTime(1768686293));
        await uow.Images.AddOrUpdateAsync(image, TestContext.Current.CancellationToken);

        var deployment = new Deployment("deployment-01", DeploymentStatus.Created, Constants.DefaultAdminId, platform.Id, UpdateBehavior.Disabled);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);

        var container = new Container(deployment.Name, "container-01-id", platform.Id, containerId, ContainerStateStatus.Created, deploymentId: deployment.Id, imageId: image.Id, created: 1768686293);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Get_Container_ReturnsSuccess()
    {
        // Act
        var request = new HttpRequestMessage(HttpMethod.Get, $"/api/v1/containers/{containerId}");

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
