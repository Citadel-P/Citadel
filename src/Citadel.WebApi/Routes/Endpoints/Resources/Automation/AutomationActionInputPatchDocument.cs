using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Automation;

public sealed class UpdateAutomationActionInputPatchDocument : JsonMergePatchDocument<UpdateAutomationActionInput>
{
    public bool ContainsProperty(string propertyName)
        => Patch.ValueKind == JsonValueKind.Object && Patch.TryGetProperty(propertyName, out _);

    public static async ValueTask<UpdateAutomationActionInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new UpdateAutomationActionInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
