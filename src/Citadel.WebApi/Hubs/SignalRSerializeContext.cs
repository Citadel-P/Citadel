using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Platforms;
using Nerdbank.MessagePack;
using PolyType;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.Images;
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
[GenerateShapeFor<ImagesView>]
[GenerateShapeFor<ImageView>]
[GenerateShapeFor<IEnumerable<ImageView>>]
[GenerateShapeFor<DockerNetworkResult>]
[GenerateShapeFor<DockerVolumeResult>]
[GenerateShapeFor<DeploymentsView>]
[GenerateShapeFor<DeploymentView>]
[GenerateShapeFor<ActivityView>]
[GenerateShapeFor<AlertEventView>]
[GenerateShapeFor<List<AlertEventView>>]
[GenerateShapeFor<UnresolvedAlertsCountView>]
partial class SignalRMessagePackContext;

internal static class DerivedTypesMapping
{
    internal static DerivedTypeMapping<PlatformDescriptor> PlatformDescriptorMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(PlatformType.Docker)] = typeof(DockerPlatformDescriptor),
        [nameof(PlatformType.Kubernetes)] = typeof(KubernetesPlatformDescriptor),
        [nameof(PlatformType.DockerSwarm)] = typeof(DockerSwarmPlatformDescriptor),
    };
}
