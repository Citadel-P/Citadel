namespace Domain.Contracts.Resources.Volumes;

public record VolumeAccessMode(
    VolumeScope Scope,
    VolumeSharing Sharing,
    IReadOnlyList<VolumeSecret> Secrets,
    VolumeCapacityRange? CapacityRange,
    string Availability);
