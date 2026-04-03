using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public class AlertRuleInputPatchDocument : JsonMergePatchDocument<PatchAlertRuleInput>
{
    public static async ValueTask<AlertRuleInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new AlertRuleInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
