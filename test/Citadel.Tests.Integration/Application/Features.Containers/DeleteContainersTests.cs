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

    [Fact]
    public async Task Delete_StaleContainerByCitadelId_ReturnsSuccessAndDeletesRow()
    {
        var platform = Fakes.GetDummyPlatform();
        const string dockerContainerId = "62ccd07956a662ccd07956a662ccd07956a662ccd07956a662ccd07956a6";
        var container = new Container(
            name: "stale-container-by-citadel-id",
            dockerImageId: "sha256:stale",
            platformId: platform.Id,
            dockerContainerId: dockerContainerId,
            state: ContainerStateStatus.Offline);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/containers")
        {
            Content = new StringContent(
                $$"""{"containerIds":["{{container.Id}}"],"force":true}""",
                Encoding.UTF8,
                "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.Null(await assertUow.Containers.GetByIdAsync(container.Id, TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_MixedSelectionContainingSystemContainer_ReturnsConflictAndDeletesNothing()
    {
        var platform = Fakes.GetDummyPlatform();
        const string systemContainerId = "12ccd07956a612ccd07956a612ccd07956a612ccd07956a612ccd07956a6";
        const string normalContainerId = "22ccd07956a622ccd07956a622ccd07956a622ccd07956a622ccd07956a6";
        var systemContainer = new Container(
            name: "citadel-server",
            dockerImageId: "sha256:citadel",
            platformId: platform.Id,
            dockerContainerId: systemContainerId,
            state: ContainerStateStatus.Running,
            isSystem: true,
            systemRole: ContainerSystemRole.Core);
        var normalContainer = new Container(
            name: "app",
            dockerImageId: "sha256:app",
            platformId: platform.Id,
            dockerContainerId: normalContainerId,
            state: ContainerStateStatus.Running);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await uow.Containers.AddAsync(systemContainer, TestContext.Current.CancellationToken);
            await uow.Containers.AddAsync(normalContainer, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var content = $$"""
        {
            "containerIds": [
                "{{systemContainerId[..12]}}",
                "{{normalContainerId[..12]}}"
            ],
            "v": false,
            "force": true,
            "link": false
        }
        """;

        var response = await Client.SendAsync(
            new HttpRequestMessage(HttpMethod.Delete, "/api/v1/containers")
            {
                Content = new StringContent(content, Encoding.UTF8, "application/json")
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("Citadel system containers cannot be managed from Citadel", responseBody);

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.NotNull(await assertUow.Containers.GetByIdAsync(systemContainerId, TestContext.Current.CancellationToken));
        Assert.NotNull(await assertUow.Containers.GetByIdAsync(normalContainerId, TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Restart_SystemContainer_ReturnsConflictWithoutChangingProcessingState()
    {
        var platform = Fakes.GetDummyPlatform();
        const string containerId = "32ccd07956a632ccd07956a632ccd07956a632ccd07956a632ccd07956a6";
        var container = new Container(
            name: "citadel-pg-db",
            dockerImageId: "sha256:postgres",
            platformId: platform.Id,
            dockerContainerId: containerId,
            state: ContainerStateStatus.Running,
            isSystem: true,
            systemRole: ContainerSystemRole.Database);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var response = await Client.SendAsync(
            new HttpRequestMessage(HttpMethod.Patch, "/api/v1/containers/restart")
            {
                Content = new StringContent($$"""["{{containerId[..12]}}"]""", Encoding.UTF8, "application/json")
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await assertUow.Containers.GetByIdAsync(containerId, TestContext.Current.CancellationToken);
        Assert.NotNull(persisted);
        Assert.Equal(ResourceControlState.Idle, persisted.ControlState);
    }

    [Fact]
    public async Task Delete_SwarmTaskContainer_ReturnsConflictAndPreservesTheTask()
    {
        var platform = Fakes.GetDummyPlatform();
        const string containerId = "52ccd07956a652ccd07956a652ccd07956a652ccd07956a652ccd07956a6";
        var container = new Container(
            name: "redis-test_web.1.task-id",
            dockerImageId: "sha256:redis",
            platformId: platform.Id,
            dockerContainerId: containerId,
            state: ContainerStateStatus.Running,
            dockerStack: "redis-test",
            isSwarmTask: true);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var response = await Client.SendAsync(
            new HttpRequestMessage(HttpMethod.Delete, "/api/v1/containers")
            {
                Content = new StringContent(
                    $$"""{"containerIds":["{{containerId[..12]}}"],"force":true}""",
                    Encoding.UTF8,
                    "application/json")
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        Assert.Contains(
            "Docker Swarm task containers cannot be managed directly",
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.NotNull(await assertUow.Containers.GetByIdAsync(containerId, TestContext.Current.CancellationToken));
    }
}
