using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Deployments;

public class DeploymentCreateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    Guid? _platformId;
    
    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        _platformId = platform.Id;
    }

    [Fact]
    public async Task Create_Deployment_WithAutoUpdateExternalImage_ReturnsSuccess()
    {
        var createJson = $$"""
            {
                "name":"deployment-1",
                "platformId":"{{_platformId}}",
                "spec":{
                    "updateBehavior":"Notify",
                    "image":{
                        "$type":"External",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx"
                     },
                     "ports":[],
                     "networks":["96da77baf016bb722c40f10c023b2f5d1a4296b4bc6593cfad74de4bd31e8b14"]
                }
            }
            """;

        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/deployments", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployments = await uow.Deployments.GetInfoAsync(TestContext.Current.CancellationToken);

        Assert.Contains(deployments, r => r.Name == "deployment-1");
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Deployment_WithAutoUpdateInternalImage_ReturnsBadRequest()
    {
        var createJson = $$"""
            {
                "name":"deployment-auto-update",
                "platformId":"{{_platformId}}",
                "spec":{
                    "updateBehavior":"AutoDeploy",
                    "image":{
                        "$type":"Internal",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx:latest"
                     },
                     "ports":[],
                     "networks":["96da77baf016bb722c40f10c023b2f5d1a4296b4bc6593cfad74de4bd31e8b14"]
                }
            }
            """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");
        // Act
        var response = await Client.PostAsync("/api/v1/deployments", content, cancellationToken: TestContext.Current.CancellationToken);
        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_Deployment_WithPinnedDigestImage_ReturnsBadRequest()
    {
        var createJson = $$"""
            {
                "name":"deployment-auto-update",
                "platformId":"{{_platformId}}",
                "spec":{
                    "updateBehavior":"AutoDeploy",
                    "image":{
                        "$type":"External",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx:latest@my-pinned-digest"
                     },
                     "ports":[],
                     "networks":["96da77baf016bb722c40f10c023b2f5d1a4296b4bc6593cfad74de4bd31e8b14"]
                }
            }
            """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");
        // Act
        var response = await Client.PostAsync("/api/v1/deployments", content, cancellationToken: TestContext.Current.CancellationToken);
        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_Deployment_WithDuplicateName_ReturnsConflict()
    {
        // Arrange
        var createJson = $$"""
            {
                "name":"deployment-duplicate",
                "platformId":"{{_platformId}}",
                "spec":{
                    "updateBehavior":"Notify",
                    "image":{
                        "$type":"External",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx"
                     },
                     "ports":[],
                     "networks":["96da77baf016bb722c40f10c023b2f5d1a4296b4bc6593cfad74de4bd31e8b14"]
                }
            }
            """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");
        // Act
        var response1 = await Client.PostAsync("/api/v1/deployments", content, cancellationToken: TestContext.Current.CancellationToken);
        var response2 = await Client.PostAsync("/api/v1/deployments", content, cancellationToken: TestContext.Current.CancellationToken);
        // Assert
        response1.EnsureSuccessStatusCode();
        Assert.Equal(System.Net.HttpStatusCode.Conflict, response2.StatusCode);
    }

}
