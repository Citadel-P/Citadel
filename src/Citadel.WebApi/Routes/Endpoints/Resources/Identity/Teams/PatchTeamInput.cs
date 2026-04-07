using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record PatchTeamInput(bool? IsEnabled);

public sealed class PatchTeamInputPatchDocument : JsonMergePatchDocument<PatchTeamInput>
{
    public static async ValueTask<PatchTeamInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new PatchTeamInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
