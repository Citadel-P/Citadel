using Domain.Contracts.Interfaces;
using Domain;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Deployments;

public class DeploymentViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid? _platformId;
    private Guid? _otherPlatformId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = CreatePlatform("deployment-view-platform-a", "https://deployment-view-a");
        var otherPlatform = CreatePlatform("deployment-view-platform-b", "https://deployment-view-b");
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(otherPlatform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        _platformId = platform.Id;
        _otherPlatformId = otherPlatform.Id;
    }

    [Fact]
    public async Task Create_Deployment_Should_Return_Forbidden_If_User_Lacks_Permission()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var createJson = $$"""
            {
                "name":"deployment-forbidden",
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

        var response = await Client.PostAsync("/api/v1/deployments", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Deployment_By_Team_User_Should_Not_Be_Viewable_By_Other_Team_Without_Access()
    {
        var creator = await CreateAuthorizationSubjectAsync(teamRoleId: OperatorRoleId);
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(creator.UserId, creator.ActorId));

        var createJson = $$"""
            {
                "name":"deployment-team-scope",
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
        var createContent = new StringContent(createJson, Encoding.UTF8, "application/json");

        var createResponse = await Client.PostAsync("/api/v1/deployments", createContent, cancellationToken: TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        Guid deploymentId;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            deploymentId = (await uow.Deployments.GetAllAsync(TestContext.Current.CancellationToken))
                .Single(x => x.Name == "deployment-team-scope")
                .Id;
        }

        var otherUser = await CreateAuthorizationSubjectAsync(createTeam: true);
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(otherUser.UserId, otherUser.ActorId));

        var response = await Client.GetAsync($"/api/v1/deployments/{deploymentId}", TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task List_Deployments_Should_Return_Only_Deployments_User_Is_Permitted_To_View()
    {
        var visibleDeploymentId = await CreateDeploymentAsync("deployment-visible");
        await CreateDeploymentAsync("deployment-hidden");

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Deployment, visibleDeploymentId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/deployments", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var deployments = document.RootElement.GetProperty("deployments");

        Assert.Equal(1, deployments.GetArrayLength());
        Assert.Equal(visibleDeploymentId, deployments[0].GetProperty("id").GetGuid());
        Assert.Equal("deployment-visible", deployments[0].GetProperty("name").GetString());
    }

    [Fact]
    public async Task List_Deployments_Should_Filter_By_Platform()
    {
        var targetDeploymentId = await CreateDeploymentAsync("deployment-platform-target", _platformId!.Value);
        await CreateDeploymentAsync("deployment-platform-other", _otherPlatformId!.Value);

        var response = await Client.GetAsync(
            $"/api/v1/deployments?platformId={_platformId!.Value:D}",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var deployment = Assert.Single(document.RootElement.GetProperty("deployments").EnumerateArray());
        Assert.Equal(targetDeploymentId, deployment.GetProperty("id").GetGuid());
        Assert.Equal(_platformId, deployment.GetProperty("platformId").GetGuid());
    }

    private async Task<Guid> CreateDeploymentAsync(string name)
        => await CreateDeploymentAsync(name, _platformId!.Value);

    private async Task<Guid> CreateDeploymentAsync(string name, Guid platformId)
    {
        var createJson = $$"""
            {
                "name":"{{name}}",
                "platformId":"{{platformId}}",
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

        var response = await Client.PostAsync(
            "/api/v1/deployments",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        return (await uow.Deployments.GetAllAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }

    private static Platform CreatePlatform(string name, string address) => new(
        name: name,
        address: address,
        networkCount: 1,
        volumeCount: 1,
        imageCount: 1,
        cpuCount: 2,
        memTotal: 512,
        serverVersion: "1.0.0",
        agentVersion: "1.0.0",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerPlatformDescriptor(
            DaemonId: name,
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0));
}
