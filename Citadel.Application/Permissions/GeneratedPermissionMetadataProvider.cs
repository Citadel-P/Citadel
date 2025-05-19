using Hosting.Common.Pipelines;
using Citadel.SourceGen;

namespace Application.Permissions;

/// <summary>
/// Used by PermissionBehavior pipeline to provide permission metadata at runtime and avoid reflection
/// </summary>
internal class GeneratedPermissionMetadataProvider : IPermissionMetadataProvider
{
    public IReadOnlyList<string> GetPermissionsFor(Type type)
    {
        if (PermissionMetadataRegistry.PermissionsByType.TryGetValue(type, out var perms))
            return perms;
        return [];
    }
}
