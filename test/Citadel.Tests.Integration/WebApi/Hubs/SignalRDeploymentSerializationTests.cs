using Domain;
using Domain.Entities.Deployments;
using Nerdbank.MessagePack;
using WebApi.Hubs;

namespace Tests.Integration.WebApi.Hubs;

public sealed class SignalRDeploymentSerializationTests
{
    [Fact]
    public void DeploymentSpec_ShouldSerializeExternalImageWithItsDiscriminator()
    {
        var serializer = new MessagePackSerializer
        {
            SerializeEnumValuesByName = true,
            PropertyNamingPolicy = MessagePackNamingPolicy.CamelCase,
            DerivedTypeUnions = [DerivedTypesMapping.DeploymentImageInfoMappings],
        };
        var spec = new DeploymentSpec(
            new ExternalImage(Guid.NewGuid(), "nginx:latest", "sha256:applied"),
            UpdateBehavior.Notify);
        var payload = serializer.Serialize<DeploymentSpec, SignalRMessagePackContext>(
            spec,
            TestContext.Current.CancellationToken);
        var json = serializer.ConvertToJson(payload);

        Assert.Contains("\"image\":[\"External\",", json, StringComparison.Ordinal);
        Assert.Contains("\"resolvedDigest\":\"sha256:applied\"", json, StringComparison.Ordinal);
    }
}
