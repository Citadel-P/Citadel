namespace Domain.Contracts.Resources.Volumes;

public sealed record InspectDockerVolumeCommand(
    string PlatformAddress,
    string Name
    );
