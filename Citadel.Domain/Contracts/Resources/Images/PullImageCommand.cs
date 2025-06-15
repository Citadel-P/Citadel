namespace Domain.Contracts.Resources.Images;

// --- Image Pulling ---
public record PullImageCommand(
    string FromImage,
    string FromSrc,
    string Repo,
    string? Tag,
    string Auth,
    IReadOnlyList<string> Changes
);

