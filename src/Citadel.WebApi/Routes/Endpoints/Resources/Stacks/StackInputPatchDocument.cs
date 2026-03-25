using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed class StackInputPatchDocument : JsonMergePatchDocument<StackInput>
{
    public static async ValueTask<StackInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new StackInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}