namespace Domain.Contracts.Resources.Images;

public record PullImageCommand(
    string PlatformAddress,
    string FromImage,
    string? FromSrc = null,
    string? Repo = null,
    string? Auth = null,
    string? Tag = null
);

