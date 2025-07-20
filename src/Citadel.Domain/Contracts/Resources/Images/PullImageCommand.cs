namespace Domain.Contracts.Resources.Images;

public record PullImageCommand(
    string PlatformAddress,
    string FromImage,
    string FromSrc,
    string Repo,
    string Auth,
    string? Tag = null,
    string? RegistryName = null
);

