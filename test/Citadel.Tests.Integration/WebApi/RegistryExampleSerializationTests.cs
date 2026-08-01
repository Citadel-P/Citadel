using WebApi.Routes.Endpoints.Resources;

namespace Tests.Integration.WebApi;

public sealed class RegistryExampleSerializationTests
{
    [Fact]
    public void RegistryExamples_ShouldSerializeStatusWithoutRuntimeEnumCodeGeneration()
    {
        var example = Examples.Registries.Create.CreateDockerHubRegistryExample();
        var json = example.Value?.ToJsonString();

        Assert.NotNull(json);
        Assert.Contains("\"Status\":\"Active\"", json, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("\"Status\":0", json, StringComparison.OrdinalIgnoreCase);
    }
}
