using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public class RegistryInputPatchDocument : JsonMergePatchDocument<RegistryInput>
{
    public static async ValueTask<RegistryInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo parameter)
    {
        //if (!context.Request.ContentType?.StartsWith("application/merge-patch+json", StringComparison.OrdinalIgnoreCase) ?? true)
        //{
        //    context.Response.StatusCode = StatusCodes.Status415UnsupportedMediaType;
        //    return null;
        //}

        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new RegistryInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
