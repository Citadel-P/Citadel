namespace Domain.Contracts.Resources.Identity;

public sealed record PatchServiceAccountModel(
    string? Description,
    bool? IsEnabled);
