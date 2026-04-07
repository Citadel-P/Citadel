using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record PatchRolePermissionsInput(IEnumerable<PermissionInput> Permissions)
{
}

public sealed class PatchRolePermissionsInputPatchDocument : JsonMergePatchDocument<PatchRolePermissionsInput>
{
    public static async ValueTask<PatchRolePermissionsInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new PatchRolePermissionsInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
