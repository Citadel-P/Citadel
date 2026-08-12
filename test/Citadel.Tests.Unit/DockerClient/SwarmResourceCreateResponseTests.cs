using System.Text.Json;
using Hosting.DockerClient;
using Hosting.DockerClient.Serializer;

namespace Tests.Unit.DockerClient;

public sealed class SwarmResourceCreateResponseTests
{
    [Fact]
    public void Deserialize_ShouldReadDockerUppercaseId()
    {
        const string json = """{"ID":"swarm-resource-id"}""";

        var response = JsonSerializer.Deserialize(
            json,
            SystemContext.Default.SwarmResourceCreateResponse);

        Assert.NotNull(response);
        Assert.Equal("swarm-resource-id", response.ID);
    }
}
