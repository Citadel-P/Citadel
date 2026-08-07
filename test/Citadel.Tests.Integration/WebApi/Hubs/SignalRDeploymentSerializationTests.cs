using Domain;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Domain.Entities.SwarmServices;
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

    [Fact]
    public void SwarmServiceSpec_ShouldSerializeExternalImageWithItsDiscriminator()
    {
        var serializer = new MessagePackSerializer
        {
            SerializeEnumValuesByName = true,
            PropertyNamingPolicy = MessagePackNamingPolicy.CamelCase,
            DerivedTypeUnions = [DerivedTypesMapping.SwarmServiceImageInfoMappings],
        };
        var spec = new SwarmServiceSpec
        {
            Image = new SwarmExternalImage(Guid.NewGuid(), "redis", "sha256:applied"),
        };
        var payload = serializer.Serialize<SwarmServiceSpec, SignalRMessagePackContext>(
            spec,
            TestContext.Current.CancellationToken);
        var json = serializer.ConvertToJson(payload);

        Assert.Contains("\"image\":[\"External\",", json, StringComparison.Ordinal);
        Assert.Contains("\"imageTag\":\"redis\"", json, StringComparison.Ordinal);
        Assert.Contains("\"resolvedDigest\":\"sha256:applied\"", json, StringComparison.Ordinal);
    }

    [Fact]
    public void SwarmServiceDuplicated_ShouldSerializeWithItsSource()
    {
        var serializer = new MessagePackSerializer
        {
            SerializeEnumValuesByName = true,
            PropertyNamingPolicy = MessagePackNamingPolicy.CamelCase,
            DerivedTypeUnions =
            [
                DerivedTypesMapping.ActivityEventInfoMappings,
                DerivedTypesMapping.SwarmServiceImageInfoMappings,
            ],
        };
        ActivityEventInfo info = new SwarmServiceDuplicated(
            new SwarmServiceActivitySnapshot(
                Guid.NewGuid(),
                Guid.NewGuid(),
                "redis-copy",
                null,
                "redis-copy-service",
                null,
                new SwarmServiceSpec
                {
                    Image = new SwarmExternalImage(Guid.NewGuid(), "redis"),
                }),
            new ActivitySourceResource(ActivityResourceType.SwarmService, Guid.NewGuid(), "redis"));
        var payload = serializer.Serialize<ActivityEventInfo, SignalRMessagePackContext>(
            info,
            TestContext.Current.CancellationToken);
        var json = serializer.ConvertToJson(payload);

        Assert.Contains("[\"SwarmServiceDuplicated\",", json, StringComparison.Ordinal);
        Assert.Contains("\"resourceName\":\"redis\"", json, StringComparison.Ordinal);
    }
}
