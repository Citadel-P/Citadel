namespace Domain.Contracts.Resources.Volumes;

public record ClusterVolume(
    string Id,
    VolumeVersionInfo? Version,
    string CreatedAt,
    string UpdatedAt,
    VolumeSpecification? Spec,
    ClusterVolumeInfo? Info,
    IReadOnlyList<VolumePublishStatus> PublishStatus);
