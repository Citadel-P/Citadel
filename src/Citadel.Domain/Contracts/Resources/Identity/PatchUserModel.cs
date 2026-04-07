namespace Domain.Contracts.Resources.Identity;

public sealed record PatchUserModel(string? Email, string? Password, bool? IsEnabled);
