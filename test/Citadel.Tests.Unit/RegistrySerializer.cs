using System.Text.Json;
using Domain;
using Domain.Entities.Registries;

namespace Tests.Unit;

public class RegistrySerializer
{
    [Fact]
    public void ShouldDeserializeDockerHubRegistryConfiguration()
    {
        var json = """
        {
            "$type":"DockerHub",
            "UserName": "username-1",
            "PAT": "fake-pat-1"
        }
        """;
        var cfg = JsonSerializer.Deserialize(json, RegistryJsonContext.Default.RegistryConfigurationBase);
        Assert.NotNull(cfg);
        Assert.IsType<DockerHubRegistry>(cfg);
        Assert.Equal("fake-pat-1", (cfg as DockerHubRegistry)?.PAT );
        Assert.Equal("username-1", (cfg as DockerHubRegistry)?.UserName );
    }

    [Fact]
    public async Task ShouldSerializeeDockerHubRegistryConfiguration()
    {
        var cfg = DockerHubRegistry.Create("username-1", "fake-pat-1");
        var json = JsonSerializer.Serialize(cfg, RegistryJsonContext.Default.RegistryConfigurationBase);
        await VerifyJson(json);
    }

    [Fact]
    public void ShouldDeserializeGitHubRegistryConfiguration()
    {
        var json = """
        {
            "$type":"GitHub",
            "Name": "username-1",
            "PAT": "fake-pat-1"
        }
        """;
        var cfg = JsonSerializer.Deserialize(json, RegistryJsonContext.Default.RegistryConfigurationBase);
        Assert.NotNull(cfg);
        Assert.IsType<GitHubRegistry>(cfg);
        Assert.Equal("fake-pat-1", (cfg as GitHubRegistry)?.PAT);
        Assert.Equal("username-1", (cfg as GitHubRegistry)?.Name);
    }

    [Fact]
    public async Task ShouldSerializeeGitHubRegistryConfiguration()
    {
        var cfg = GitHubRegistry.Create(true, "username-1", "fake-pat-1", GhcrAccountType.User);
        var json = JsonSerializer.Serialize(cfg, RegistryJsonContext.Default.RegistryConfigurationBase);
        await VerifyJson(json);
    }
}
