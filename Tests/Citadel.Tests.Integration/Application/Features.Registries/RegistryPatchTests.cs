using System.Text;
using Domain;
using Domain.Entities;
using Domain.Entities.Registries;
using Infrastructure.EntityFramework;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Registries;

public class RegistryPatchTests : IntegrationTestBase<WebApi.Program>
{
    private Guid registryId;
    protected override async ValueTask SeedDbAsync()
    {
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var registry = new Registry(
            name: "OriginalName",
            url: "https://original.url",
            type: RegistryType.DockerHub,
            configuration: new DockerHubRegistry("original-user", "pat123")
        );
        db.Registries.Add(registry);

        await db.SaveChangesAsync();

        registryId = registry.Id;
    }

    [Fact]
    public async Task Patch_Registry_Should_Apply_MergePatch()
    {
        // Arrange
        var patchJson = """
        {
          "name": "UpdatedName",
          "type": "DockerHub",
          "configuration": {
            "$type": "DockerHub",
            "userName": "patched-user"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}