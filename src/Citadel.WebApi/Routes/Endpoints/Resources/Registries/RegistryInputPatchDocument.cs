using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public class RegistryInputPatchDocument : JsonMergePatchDocument<PatchRegistryInput>
{
    public static async ValueTask<RegistryInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new RegistryInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
