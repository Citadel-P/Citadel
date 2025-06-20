namespace Domain.Contracts.Resources.Volumes;

public record CreateDockerVolumeCommand(
    string PlatformAddress,
    string Name,
    string Driver,
    IReadOnlyDictionary<string, string>? Labels,
    IReadOnlyDictionary<string, string>? Options);
