namespace Domain.Contracts.Resources.Volumes;

public sealed record DeleteDockerVolumeCommand(string PlatformAddress, bool? Force, IReadOnlyList<string> Names);