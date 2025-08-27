using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Nerdbank.MessagePack;
using PolyType;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

[GenerateShapeFor<ContainersView>]
[GenerateShapeFor<ContainerView>]
[GenerateShapeFor<List<ContainerView>>]
[GenerateShapeFor<List<ContainerStatView>>]
[GenerateShapeFor<ContainerStatView>]
[GenerateShapeFor<Dictionary<string, string>>]
[GenerateShapeFor<IEnumerable<PlatformView>>]
[GenerateShapeFor<PlatformView>]
[GenerateShapeFor<KubernetesPlatformDescriptor>]
[GenerateShapeFor<DockerSwarmPlatformDescriptor>]
[GenerateShapeFor<DockerPlatformDescriptor>]
[GenerateShapeFor<List<PlatformStatView>>]
[GenerateShapeFor<SwarmInfoView>]
[GenerateShapeFor<List<SwarmPeerView>>]
[GenerateShapeFor<PlatformsView>]
[GenerateShapeFor<PlatformStatsBatchView>]
[GenerateShapeFor<PlatformStatView>]
[GenerateShapeFor<DockerContainer>]
[GenerateShapeFor<DockerContainerStat>]
[GenerateShapeFor<byte[]>]
[GenerateShapeFor<ReadOnlyMemory<byte>>]
[GenerateShapeFor<IDictionary<string, IReadOnlyList<HostPortBinding>>>]
partial class SignalRMessagePackContext;

internal static class DerivedTypesMapping
{
    internal static DerivedTypeMapping<PlatformDescriptor> PlatformDescriptorMappings = new(SignalRMessagePackContext.ShapeProvider)
    {
        [nameof(PlatformType.Docker)] = typeof(DockerPlatformDescriptor),
        [nameof(PlatformType.Kubernetes)] = typeof(KubernetesPlatformDescriptor),
        [nameof(PlatformType.DockerSwarm)] = typeof(DockerSwarmPlatformDescriptor),
    };
}
