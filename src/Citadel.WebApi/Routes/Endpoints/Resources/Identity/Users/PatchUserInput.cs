using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record PatchUserInput(
    string? Email,
    string? Password,
    bool? IsEnabled,
    IEnumerable<Guid>? TeamIds,
    IEnumerable<Guid>? RoleIds,
    IEnumerable<UserResourceAccessInput>? ResourceAccesses);

public sealed class PatchUserInputPatchDocument : JsonMergePatchDocument<PatchUserInput>
{
    public static async ValueTask<PatchUserInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new PatchUserInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
