namespace Domain.Contracts.Resources.Compose;

public sealed record ComposeUpCommand(
    string PlatformAddress,
    string RegistryName,
    string ComposeFileAsStr
    );
