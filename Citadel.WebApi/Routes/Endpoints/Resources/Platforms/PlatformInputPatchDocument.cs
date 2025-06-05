using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public class PlatformInputPatchDocument : JsonMergePatchDocument<PlatformInput>
{
    public static async ValueTask<PlatformInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new PlatformInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
