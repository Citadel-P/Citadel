using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Nerdbank.MessagePack;
using PolyType;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
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
[GenerateShapeFor<GitRepositoryView>]
[GenerateShapeFor<LatestActivityView>]
[GenerateShapeFor<DeploymentCreated>]
[GenerateShapeFor<DeploymentUpdated>]
[GenerateShapeFor<DeploymentRenamed>]
[GenerateShapeFor<DeploymentDeleted>]
[GenerateShapeFor<DeploymentStarted>]
[GenerateShapeFor<DeploymentStopped>]
[GenerateShapeFor<DeploymentPaused>]
[GenerateShapeFor<DeploymentApplied>]
[GenerateShapeFor<DeploymentDegraded>]
[GenerateShapeFor<AlertRuleCreated>]
[GenerateShapeFor<AlertRuleUpdated>]
[GenerateShapeFor<AlertRuleDeleted>]
[GenerateShapeFor<AlertRuleRenamed>]
[GenerateShapeFor<RegistryRenamed>]
[GenerateShapeFor<RegistryCreated>]
[GenerateShapeFor<RegistryUpdated>]
[GenerateShapeFor<RegistryDeleted>]
[GenerateShapeFor<GitRepoCreated>]
[GenerateShapeFor<GitRepoUpdated>]
[GenerateShapeFor<GitRepoRenamed>]
[GenerateShapeFor<GitRepoDeleted>]
[GenerateShapeFor<GitRepoCloned>]
[GenerateShapeFor<GitRepoPulled>]
partial class SignalRMessagePackContext;

internal static class DerivedTypesMapping
{
    internal static DerivedTypeMapping<PlatformDescriptor> PlatformDescriptorMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(PlatformType.Docker)] = typeof(DockerPlatformDescriptor),
        [nameof(PlatformType.Kubernetes)] = typeof(KubernetesPlatformDescriptor),
        [nameof(PlatformType.DockerSwarm)] = typeof(DockerSwarmPlatformDescriptor),
    };

    internal static DerivedTypeMapping<ActivityEventInfo> ActivityEventInfoMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(ActivityEventType.DeploymentCreated)] = typeof(DeploymentCreated),
        [nameof(ActivityEventType.DeploymentUpdated)] = typeof(DeploymentUpdated),
        [nameof(ActivityEventType.DeploymentRenamed)] = typeof(DeploymentRenamed),
        [nameof(ActivityEventType.DeploymentDeleted)] = typeof(DeploymentDeleted),
        [nameof(ActivityEventType.DeploymentStarted)] = typeof(DeploymentStarted),
        [nameof(ActivityEventType.DeploymentStopped)] = typeof(DeploymentStopped),
        [nameof(ActivityEventType.DeploymentPaused)] = typeof(DeploymentPaused),
        [nameof(ActivityEventType.DeploymentApplied)] = typeof(DeploymentApplied),
        [nameof(ActivityEventType.DeploymentDegraded)] = typeof(DeploymentDegraded),
        [nameof(ActivityEventType.AlertRuleCreated)] = typeof(AlertRuleCreated),
        [nameof(ActivityEventType.AlertRuleUpdated)] = typeof(AlertRuleUpdated),
        [nameof(ActivityEventType.AlertRuleDeleted)] = typeof(AlertRuleDeleted),
        [nameof(ActivityEventType.AlertRuleRenamed)] = typeof(AlertRuleRenamed),
        [nameof(ActivityEventType.RegistryRenamed)] = typeof(RegistryRenamed),
        [nameof(ActivityEventType.RegistryCreated)] = typeof(RegistryCreated),
        [nameof(ActivityEventType.RegistryUpdated)] = typeof(RegistryUpdated),
        [nameof(ActivityEventType.RegistryDeleted)] = typeof(RegistryDeleted),
        [nameof(ActivityEventType.GitRepoCreated)] = typeof(GitRepoCreated),
        [nameof(ActivityEventType.GitRepoUpdated)] = typeof(GitRepoUpdated),
        [nameof(ActivityEventType.GitRepoRenamed)] = typeof(GitRepoRenamed),
        [nameof(ActivityEventType.GitRepoDeleted)] = typeof(GitRepoDeleted),
        [nameof(ActivityEventType.GitRepoCloned)] = typeof(GitRepoCloned),
        [nameof(ActivityEventType.GitRepoPulled)] = typeof(GitRepoPulled),
    };
}
