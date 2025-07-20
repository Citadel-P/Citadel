namespace Domain.Contracts.Resources.Volumes;

public record ListdDockerVolumesCommand(
    string PlatformAddress,
    bool? Dangling,
    string? Driver,
    string? Name
);
