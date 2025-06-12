namespace Domain.Contracts.Resources.Containers;

public sealed record StreamDaemonEventCommand(Guid PlatformId, string PlatformAddress);
