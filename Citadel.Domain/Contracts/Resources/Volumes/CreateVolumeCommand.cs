namespace Domain.Contracts.Resources.Volumes;

public record CreateVolumeCommand(
    string PlatformAddress,
    string Name,
    string Driver,
    IReadOnlyDictionary<string, string>? Labels,
    IReadOnlyDictionary<string, string>? Options);
