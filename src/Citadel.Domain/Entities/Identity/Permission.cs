using Hosting.Common;
using Hosting.Common.Attributes;

namespace Domain.Entities.Identity;

public class Permission
{
    private const int KnownSpecificPermissionsMask =
        (int)SpecificPermission.Apply |
        (int)SpecificPermission.Logs |
        (int)SpecificPermission.Terminal |
        (int)SpecificPermission.Pull;

    public Guid Id { get; private set; }
    public Guid RoleId { get; private set; }
    public ResourceType ResourceType { get; private set; }
    public PermissionLevel PermissionLevel { get; private set; }
    public IReadOnlyList<SpecificPermission> SpecificPermissions { get; private set; } = [];

    public static Permission Create(
        Guid roleId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions = null,
        Guid? id = null)
    {
        var normalizedSpecificPermissions = NormalizeSpecificPermissions(resourceType, permissionLevel, specificPermissions);

        return new()
        {
            Id = id ?? Guid.CreateVersion7(),
            RoleId = roleId,
            ResourceType = resourceType,
            PermissionLevel = permissionLevel,
            SpecificPermissions = normalizedSpecificPermissions,
        };
    }

    internal static SpecificPermission[] NormalizeSpecificPermissions(
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions)
    {
        var normalized = (specificPermissions ?? [])
            .Where(x => x != SpecificPermission.None)
            .Distinct()
            .OrderBy(x => x)
            .ToArray();

        if (!PermissionMatrix.IsAllowed(resourceType, permissionLevel, normalized))
            throw new InvalidOperationException(
                $"Invalid permission assignment: [{resourceType}]-[{permissionLevel}] with specifics [{string.Join(", ", normalized)}].");

        return normalized;
    }

    public static int ToSpecificPermissionsMask(IEnumerable<SpecificPermission>? specificPermissions)
    {
        if (specificPermissions is null)
            return 0;

        var mask = 0;
        foreach (var permission in specificPermissions)
        {
            if (permission == SpecificPermission.None)
                continue;

            mask |= (int)permission;
        }

        return mask;
    }

    public static SpecificPermission[] FromSpecificPermissionsMask(int mask)
    {
        if (mask == 0)
            return [];

        var unknownMask = mask & ~KnownSpecificPermissionsMask;
        if (unknownMask != 0)
            throw new InvalidOperationException($"Invalid specific permissions bitmask: {mask}.");

        var permissions = new List<SpecificPermission>(4);

        if ((mask & (int)SpecificPermission.Apply) != 0)
            permissions.Add(SpecificPermission.Apply);

        if ((mask & (int)SpecificPermission.Logs) != 0)
            permissions.Add(SpecificPermission.Logs);

        if ((mask & (int)SpecificPermission.Terminal) != 0)
            permissions.Add(SpecificPermission.Terminal);

        if ((mask & (int)SpecificPermission.Pull) != 0)
            permissions.Add(SpecificPermission.Pull);

        return [.. permissions];
    }
}