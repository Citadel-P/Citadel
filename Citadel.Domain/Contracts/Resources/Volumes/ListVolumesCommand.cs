namespace Domain.Contracts.Resources.Volumes;

public record ListVolumesCommand(
    string PlatformAddress,
    bool? Dangling,
    string? Driver,
    string? Name
);
