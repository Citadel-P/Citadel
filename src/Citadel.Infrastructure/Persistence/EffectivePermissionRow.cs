namespace Infrastructure.Persistence;

internal sealed record EffectivePermissionRow(
    Guid ResourceId,
    int PermissionLevel,
    int SpecificPermissions);
