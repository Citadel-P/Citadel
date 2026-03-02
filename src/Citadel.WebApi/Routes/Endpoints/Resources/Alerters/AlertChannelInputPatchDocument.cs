using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public class AlertChannelInputPatchDocument : JsonMergePatchDocument<AlertChannelInput>
{
    public static async ValueTask<AlertChannelInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new AlertChannelInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
