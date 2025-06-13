namespace Domain.Contracts.Resources.Volumes;

public sealed record DeleteVolumeCommand(string PlatformAddress, bool? Force, IReadOnlyList<string> Names);