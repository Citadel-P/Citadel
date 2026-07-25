using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;

namespace Application.Services.Licensing;

internal static class LicenseAccessControlPolicy
{
    public static bool ExpandsPermissions(
        IEnumerable<Permission> current,
        IEnumerable<Permission> proposed)
    {
        var currentPermissions = current.ToArray();
        return proposed.Any(candidate => !currentPermissions.Any(existing =>
            existing.ResourceType == candidate.ResourceType
            && Covers(existing.PermissionLevel, candidate.PermissionLevel)
            && ContainsAll(existing.SpecificPermissions, candidate.SpecificPermissions)));
    }

    public static bool ExpandsResourceAccess(
        IEnumerable<ResourceAccess> current,
        IEnumerable<ResourceAccess> proposed)
    {
        var currentAccess = current.ToArray();
        return proposed.Any(candidate => !currentAccess.Any(existing =>
            existing.ResourceType == candidate.ResourceType
            && existing.ResourceId == candidate.ResourceId
            && Covers(existing.PermissionLevel, candidate.PermissionLevel)
            && ContainsAll(existing.SpecificPermissions, candidate.SpecificPermissions)));
    }

    private static bool Covers(PermissionLevel existing, PermissionLevel proposed)
    {
        if (proposed == PermissionLevel.None)
            return true;
        if (existing == PermissionLevel.None)
            return false;

        var grantedMask = PermissionHelpers.GetGrantedPermissionMask(proposed);
        return (grantedMask & (int)existing) != 0;
    }

    private static bool ContainsAll(
        IEnumerable<SpecificPermission> existing,
        IEnumerable<SpecificPermission> proposed)
    {
        var existingMask = Permission.ToSpecificPermissionsMask(existing);
        var proposedMask = Permission.ToSpecificPermissionsMask(proposed);
        return (existingMask & proposedMask) == proposedMask;
    }
}
