using System.Text;
using Infrastructure;
using Infrastructure.Entities;
using Infrastructure.Entities.Platforms;
using Infrastructure.EntityFramework;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformPatchTests : IntegrationTestBase<WebApi.Program>
{
    private Guid platformId;

    protected override async ValueTask SeedDbAsync()
    {
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var platform = new Platform(
            name: "P-01",
            address: "https://original.address",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            type: PlatformType.Docker,
            status: PlatformStatus.Online,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "123456",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1)
        );
        db.Platforms.Add(platform);

        await db.SaveChangesAsync();

        platformId = platform.Id;
    }

    [Fact]
    public async Task Patch_Platform_Should_Apply_MergePatch()
    {
        // Arrange
        var patchJson = """
        {
          "name": "P-02",
          "type": "Docker",
          "address": "https://localhost:9000"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");
        
        // Act
        var response = await Client.PatchAsync($"/api/v1/platforms/{platformId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
