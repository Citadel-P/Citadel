using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Deployments;

public class DeploymentPatchTests : IntegrationTestBase
{
    Guid _platformId;
    Guid _deploymentId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        var deployment = new Deployment
        (
            name: "Test Deployment",
            description: "A deployment for testing",
            platformId: platform.Id,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec
            (
                UpdateBehavior: UpdateBehavior.AutoDeploy,
                Image: new ExternalImage
                (
                    RegistryId: Constants.DefaultRegistryId,
                    ImageTag: "nginx:latest"
                ),
                Ports: new List<string> { "80:80" },
                EnvVars: new List<string> { "ENV=production" }
            )

        );

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        _platformId = platform.Id;
        _deploymentId = deployment.Id;
    }

    [Fact]
    public async Task Patch_Deployment_Should_Apply_MergePatch()
    {
        // Arrange
        var patchJson = $$"""
        {
          "name":"Updated-Deployment-Name",
          "description":"Updated description for testing",
          "platformId":"{{_platformId}}",
          "spec": 
          {
            "updateBehavior":"Disabled",
            "image":
            {
                "$type":"Local",
                "registryId":"{{Constants.DefaultRegistryId}}",
                "imageTag":"nginx",
                "imageId":"019b6552-be84-7649-a6b0-c3a73a2df59c"
            },
            "ports":["2220-27017/tcp"],
            "networks":["96da77baf016bb722c40f10c023b2f5d1a4296b4bc6593cfad74de4bd31e8b14"],
            "resourceSpec":
            {
                "nanoCpus":0.25,
                "memoryLimit":256
            },
            "lifeCycleSpec":
            {
                "stopSignal":"SIGKILL",
                "stopTimeout":15
            },
            "labels":
            {
                "key1":"val1"
            },
            "envVars":
            [
                "ENV=staging",
                "DEBUG=true"
            ],
            "command":["--housekeeping_interval=5s"]
           }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");
        // Act
        var response = await Client.PatchAsync($"/api/v1/deployments/{_deploymentId}", content, cancellationToken: TestContext.Current.CancellationToken);
        
        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);

        Assert.Equal("Updated-Deployment-Name", deployment?.Name);
        Assert.Equal("Updated description for testing", deployment?.Description);
        Assert.Contains("ENV=staging", deployment?.Spec?.EnvVars ?? []);
        Assert.Contains("DEBUG=true", deployment?.Spec?.EnvVars ?? []);
        Assert.Contains("key1", deployment?.Spec?.Labels?? []);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Deployment_DisableAutoUpdate_Success()
    {
        var patchJson = """
        {
            "spec": 
              {
                "updateBehavior": "disabled"
              }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");
        // Act
        var response = await Client.PatchAsync($"/api/v1/deployments/{_deploymentId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);

        Assert.Equal(UpdateBehavior.Disabled, deployment?.Spec?.UpdateBehavior);
    }

    [Fact]
    public async Task Patch_Deployment_WithAutoUpdateInternalImage_ReturnsBadRequest()
    {
        var createJson = $$"""
            {
                
                "spec":{
                    "image":{
                        "$type":"Internal",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx:latest"
                     }
                }
            }
            """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");
        // Act
        var response = await Client.PatchAsync($"/api/v1/deployments/{_deploymentId}", content, cancellationToken: TestContext.Current.CancellationToken);
        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_Deployment_WithPinnedDigestImage_ReturnsBadRequest()
    {
        var createJson = $$"""
            {
                "spec":{
                    "image":{
                        "$type":"External",
                        "registryId":"{{Constants.DefaultRegistryId}}",
                        "imageTag":"nginx:latest@my-pinned-digest"
                     }
                    
                }
            }
            """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");
        // Act
        var response = await Client.PatchAsync($"/api/v1/deployments/{_deploymentId}", content, cancellationToken: TestContext.Current.CancellationToken);
        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }
}
