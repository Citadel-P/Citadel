using Hosting.Common;
using Hosting.Common.Attributes;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record PermissionMatrixViewItem(
    string MaximumLevel,
    IReadOnlyDictionary<string, string> SpecificPermissions,
    string Label,
    IReadOnlyDictionary<string, string> SpecificPermissionLabels);

public static class PermissionMatrixView
{
    public static IReadOnlyDictionary<string, PermissionMatrixViewItem> Map()
        => PermissionMatrix.GetAll()
            .ToDictionary(
                kvp => kvp.Key.ToString(),
                kvp => new PermissionMatrixViewItem(
                    kvp.Value.MaximumLevel.ToString(),
                    kvp.Value.SpecificPermissionMinimumLevels.ToDictionary(
                        entry => entry.Key.ToString(),
                        entry => entry.Value.ToString()),
                    GetResourceLabel(kvp.Key),
                    kvp.Value.SpecificPermissionMinimumLevels.ToDictionary(
                        entry => entry.Key.ToString(),
                        entry => GetSpecificPermissionLabel(entry.Key))));

    private static string GetResourceLabel(ResourceType resourceType)
        => resourceType switch
        {
            ResourceType.Binding => "Bindings",
            ResourceType.GitRepository => "Git Repository",
            ResourceType.GitAccount => "Git Account",
            ResourceType.AlertChannel => "Alert Channel",
            _ => resourceType.ToString(),
        };

    private static string GetSpecificPermissionLabel(SpecificPermission permission)
        => permission switch
        {
            SpecificPermission.ResourceBindings => "Resource Bindings",
            _ => permission.ToString(),
        };
}
