using System.Text;
using Infrastructure;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Features.Registries;

public class RegistryPatchTests : IntegrationTestBase<WebApi.Program>
{
    private Guid _registryId;

    public override async ValueTask SeedDb()
    {
        // Seed db
        var db = Services.GetRequiredService<ApplicationDbContext>();
        var registry = new Registry(
            name: "OriginalName",
            url: "https://original.url",
            type: RegistryType.DockerHub,
            configuration: new DockerHubRegistry("original-user", "pat123")
        );
        db.Registries.Add(registry);

        await db.SaveChangesAsync();

        _registryId = registry.Id;
    }

    [Fact]
    public async Task Patch_Registry_Should_Apply_MergePatch()
    {
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

        var response = await Client.PatchAsync($"/api/v1/registries/{_registryId}", content);
        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync();
        await VerifyJson(responseBody);
    }
}