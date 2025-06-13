namespace Domain.Contracts.Resources.Volumes;

public sealed record InspectVolumeCommand(
    string PlatformAddress,
    string Name
    );
