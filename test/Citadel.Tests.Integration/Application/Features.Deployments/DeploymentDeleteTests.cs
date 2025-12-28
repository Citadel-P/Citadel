using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System;
using System.Collections.Generic;
using System.Text;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Deployments;

public class DeploymentDeleteTests : IntegrationTestBase
{
    private Guid deploymentId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        var deployment = new Deployment
        (
            name: "Test Deployment",
            description: "A deployment for testing",
            platformId: platform.Id,
            status: DeploymentStatus.Created,
            createdByActorId: Constants.SystemId,
            updateBehavior: UpdateBehavior.AutoDeploy,
            spec: new DeploymentSpec
            (
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
        deploymentId = deployment.Id;
    }

    [Fact]
    public async Task Delete_Deployment_ReturnsSuccess()
    {
        // Arrange
        var content = $$"""
        {
            "ids": ["{{deploymentId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/deployments")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployments = await uow.Deployments.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Empty(deployments);
        // Response has no content
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }
}
