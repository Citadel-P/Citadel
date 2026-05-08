using Hosting.Common.Attributes;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record PermissionMatrixViewItem(
    string MaximumLevel,
    IReadOnlyDictionary<string, string> SpecificPermissions);

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
                        entry => entry.Value.ToString())));
}
