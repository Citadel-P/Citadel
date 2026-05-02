namespace Domain.Contracts.Resources.Identity;

public sealed record TeamDetails(
    Guid Id,
    string Name,
    Guid ActorId,
    bool IsEnabled,
    int? TotalMembers = 0,
    IEnumerable<string>? Roles = null);
