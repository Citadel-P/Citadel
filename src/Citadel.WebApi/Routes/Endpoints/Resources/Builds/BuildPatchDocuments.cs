using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Builds;

public sealed class UpdateBuildProjectInputPatchDocument : JsonMergePatchDocument<UpdateBuildProjectInput>
{
    public bool ContainsProperty(string propertyName)
        => Patch.ValueKind == JsonValueKind.Object && Patch.TryGetProperty(propertyName, out _);

    public static async ValueTask<UpdateBuildProjectInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new UpdateBuildProjectInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
